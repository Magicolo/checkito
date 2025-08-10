use crate::utility::cast_ref;
use core::{
    any::Any,
    iter,
    marker::PhantomData,
    mem::swap,
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Range},
    panic::RefUnwindSafe,
    pin::Pin,
    ptr::null,
    sync::atomic::{AtomicUsize, Ordering},
};
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use std::{
    panic::{catch_unwind, resume_unwind},
    rc::Rc,
    sync::{Arc, Mutex, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError, Weak},
    thread::{available_parallelism, spawn},
};

/*
TODO:
- Can I extract to a library *only* only for the lifetime extension mechanism?
fn extend_fn<'a, F: Fn() + 'a>() -> Handle {}
fn extend_fn_once<'a, F: FnOnce() + 'a>() -> Handle {}
fn extend_fn_mut<'a, F: FnMut() + 'a>() -> Handle {}

- It is never safe to give a `&'static T` from a `&'a T`, even within a closure, since static references may escape the closure.
- Even when hidden within a data handle, it must be known that it the handle may escape the closure.
- Call the crate `dalitafe` for the `Data`, `Life` splitter.

Evil things that must be impossible:
- Storing an extended lifetime value in a `static` field.
    - Unsized values prevent most of the issues...
    - `Split` and `SplitMut` traits should be `unsafe`.
    static EVIL: OnceLock<Arc<Mutex<dyn Fn() + Send + Sync + 'static>>> = OnceLock::new();
    let (mut data, life) = split_mut::<dyn Fn() + Send + Sync + 'static>(f);
    let evil = EVIL.get_or_init(|| Arc::new(Mutex::new(|| {})));
    swap(
        data.borrow_mut().deref_mut(),
        evil.lock().unwrap().deref_mut(),
    );
- *NEVER* allow giving out a `static` reference to the inner type even though its lifetime is `'static`.
*/
struct Data<T>(Arc<RwLock<Option<T>>>);
struct Life<'a> {
    data: *const (),
    drop: unsafe fn(*const ()) -> bool,
    _marker: PhantomData<&'a ()>,
}
struct Ref<T: ?Sized>(*const T);
struct Mut<T: ?Sized>(*mut T);
struct DataRefGuard<'a, T: ?Sized>(RwLockReadGuard<'a, T>);
struct DataMutGuard<'a, T: ?Sized>(RwLockWriteGuard<'a, T>);

unsafe impl<T: Send + ?Sized> Send for Ref<T> {}
unsafe impl<T: Sync + ?Sized> Sync for Ref<T> {}
unsafe impl<T: Send + ?Sized> Send for Mut<T> {}
unsafe impl<T: Sync + ?Sized> Sync for Mut<T> {}

trait Split<T: ?Sized> {
    fn split(&self) -> (Data<Ref<T>>, Life<'_>) {
        todo!()
    }
}

trait SplitMut<T: ?Sized>: Split<T> {
    fn split_mut(&mut self) -> (Data<Mut<T>>, Life<'_>) {
        todo!()
    }
}

// impl<'a, T: 'a> Split<'a, &'static T> for &T {}
// impl<'a, T: 'a> Split<'a, &'static mut T> for &mut T {}
// impl<'a, T: 'a, S: Split<'a, T>> Split<'a, Pin<T>> for Pin<S> {}

// impl<T: 'static> Split<'static, dyn Any> for T {}
// impl<T: Send + 'static> Split<'static, dyn Any + Send> for T {}
// impl<T: Sync + 'static> Split<'static, dyn Any + Sync> for T {}
// impl<T: Send + Sync + 'static> Split<'static, dyn Any + Send + Sync> for T {}

impl<F: FnOnce() + Send + Sync> Split<dyn FnOnce() + Send + Sync> for F {}
impl<F: FnOnce() + Send> Split<dyn FnOnce() + Send> for F {}
impl<F: FnOnce() + Sync> Split<dyn FnOnce() + Sync> for F {}
impl<F: FnOnce()> Split<dyn FnOnce()> for F {}

impl<F: FnOnce() + Send + Sync> SplitMut<dyn FnOnce() + Send + Sync> for F {}
impl<F: FnOnce() + Send> SplitMut<dyn FnOnce() + Send> for F {}
impl<F: FnOnce() + Sync> SplitMut<dyn FnOnce() + Sync> for F {}
impl<F: FnOnce()> SplitMut<dyn FnOnce()> for F {}

impl<F: Fn() + Send + Sync> Split<dyn Fn() + Send + Sync> for F {
    fn split(&self) -> (Data<Ref<dyn Fn() + Send + Sync>>, Life<'_>) {
        let data = Arc::new(RwLock::new(Some(Ref(self as *const _ as _))));
        let weak = Arc::downgrade(&data);
        (
            Data(data),
            Life {
                data: Weak::<RwLock<Option<Ref<dyn Fn() + Send + Sync>>>>::into_raw(weak) as _,
                drop: |data| {
                    let weak = unsafe {
                        Weak::<RwLock<Option<Ref<dyn Fn() + Send + Sync>>>>::from_raw(data as _)
                    };
                    // If the upgrade fails, it means that no data handle remain.
                    if let Some(data) = weak.upgrade() {
                        // Although one might consider using `try_write` and panicking on a
                        // `WouldBlock`, this is not sufficient to prevent undefined
                        // behavior since the `panic` may leave another thread with an
                        // expired data handle. Therefore, this thread *must* acquire an
                        // exclusive data lock.
                        match data.write() {
                            Ok(mut data) => data.take().is_some(),
                            Err(mut error) => error.get_mut().take().is_some(),
                        }
                    } else {
                        true
                    }
                },
                _marker: PhantomData,
            },
        )
    }
}

impl<F: Fn() + Send + Sync> SplitMut<dyn Fn() + Send + Sync> for F {}

impl Life<'_> {
    pub fn is_alive(&self) -> bool {
        Weak::strong_count(&unsafe { Weak::<()>::from_raw(self.data) }) > 0
    }
}

impl Drop for Life<'_> {
    fn drop(&mut self) {
        unsafe { (self.drop)(self.data) };
    }
}

impl<T> Split<T> for T {
    fn split(&self) -> (Data<Ref<T>>, Life<'_>) {
        panic!("splitting sized types is highly unsafe, thus not supported")
    }
}

impl<T> SplitMut<T> for T {
    fn split_mut(&mut self) -> (Data<Mut<T>>, Life<'_>) {
        panic!("splitting sized types is highly unsafe, thus not supported")
    }
}

impl<T: ?Sized> Deref for DataRefGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> Deref for DataMutGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> DerefMut for DataMutGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: ?Sized> Data<Ref<T>> {
    fn borrow(&self) -> DataRefGuard<T> {
        todo!()
    }

    fn try_borrow(&self) -> Option<DataRefGuard<T>> {
        todo!()
    }
}

impl<T: ?Sized> Data<Mut<T>> {
    fn borrow(&self) -> DataRefGuard<T> {
        todo!()
    }

    fn borrow_mut(&self) -> DataMutGuard<T> {
        todo!()
    }

    fn try_borrow(&self) -> Option<DataRefGuard<T>> {
        todo!()
    }

    fn try_borrow_mut(&self) -> Option<DataMutGuard<T>> {
        todo!()
    }
}

fn split<T: ?Sized>(value: &impl Split<T>) -> (Data<Ref<T>>, Life<'_>) {
    value.split()
}

fn split_mut<T: ?Sized>(value: &mut impl SplitMut<T>) -> (Data<Mut<T>>, Life<'_>) {
    value.split_mut()
}

// fn karl() {
//     let mut a = 'a';
//     let (d, l) = split_box::<dyn FnOnce() + Send>(move || a = 'b');
//     drop(l);
//     if let Some(mut d) = d.try_borrow_mut() {
//         d();
//     }
// }

fn scopeth<'a, F: Fn() + Send + Sync + 'a>(f: &'a mut F) -> Life<'a> {
    use std::thread::spawn;
    let (data, life) = split::<dyn Fn() + Send + Sync + 'static>(f);
    spawn(move || {
        data.borrow().deref()();
    });
    life
}

pub struct Iterator<'a, T> {
    task: Weak<dyn Task + Send + Sync + 'a>,
    item: Option<Receiver<Item<T>>>,
    error: Receiver<Option<String>>,
}

/// A yield object which *can* and *must* be used exactly once per call of its
/// iterator closure and will produce a [`Token<T>`] as an outcome. It is the
/// *only* way to produce a [`Token<T>`].
///
/// The reason for this pattern instead of simply returning a value from the
/// iterator closure is to allow synchronization to happen around yielding
/// items (ex: using a lock), which isn't feasible with a return pattern. This
/// is particularly useful to guarantee some ordering of the iterator values.
pub struct Yield<'a, T>(&'a Sender<Item<T>>);

/// A token given as an outcome of a [`Yield<T>`] operation. It is meant to
/// ensure that [`Yield<T>`] is called exactly once per call of the iterator.
///
/// The lifetime `'a` survives in the token to prevent exchanging tokens between
/// iterators.
pub struct Token<'a>(bool, PhantomData<&'a ()>);

pub struct Pool(Arc<State>);

pub struct Executor {
    state: Arc<State>,
    parallelism: NonZeroUsize,
}

struct Message {
    task: StrongTask,
    index: usize,
    count: usize,
    error: Sender<Option<String>>,
}

struct State {
    send: Sender<Message>,
    receive: Receiver<Message>,
    ready: AtomicUsize,
    size: Range<AtomicUsize>,
}

enum Item<T> {
    Next(T),
    Last(T),
    Done,
}

type StrongTask = Arc<dyn Task + Send + Sync + 'static>;
type WeakTask = Weak<dyn Task + Send + Sync + 'static>;

trait Task: RefUnwindSafe {
    /// Will be called once per thread in the pool providing the `index` of the
    /// thread.
    fn run(&self, index: usize, count: usize);
    fn done(&self) -> bool;
}

impl Message {
    pub fn next(&self) -> Option<Self> {
        let index = self.index + 1;
        if index < self.count {
            Some(Message {
                task: self.task.clone(),
                index,
                count: self.count,
                error: self.error.clone(),
            })
        } else {
            None
        }
    }
}

impl Pool {
    pub fn global() -> &'static Self {
        static POOL: OnceLock<Pool> = OnceLock::new();
        POOL.get_or_init(|| Pool::new(None))
    }

    pub fn new(size: Option<usize>) -> Self {
        Self(Arc::new(State::new(size)))
    }

    pub fn with(self, size: usize) -> Self {
        self.0.size.end.store(size, Ordering::Relaxed);
        Self(self.0)
    }

    pub fn executor(&self, parallelism: Option<NonZeroUsize>) -> Executor {
        Executor::new(self.0.clone(), parallelism)
    }
}

impl State {
    fn new(size: Option<usize>) -> Self {
        let (send, receive) = unbounded();
        Self {
            send,
            receive,
            ready: AtomicUsize::new(0),
            size: AtomicUsize::new(0)..AtomicUsize::new(self::size(size)),
        }
    }

    fn ensure(self: &Arc<Self>, parallelism: usize) {
        let mut ready = self.ready.load(Ordering::Relaxed);
        while ready < parallelism {
            match self.next() {
                Some(index) => ready = Self::spawn(self, index),
                None => break,
            }
        }
    }

    fn send(
        self: &Arc<Self>,
        strong: StrongTask,
        parallelism: NonZeroUsize,
    ) -> (WeakTask, Receiver<Option<String>>) {
        self.ensure(parallelism.get());

        // A weak reference is returned to allow the threads to drop the task as soon as
        // it's done.
        let weak = Arc::downgrade(&strong);
        let (send, receive) = bounded(1);
        let mut outer = Some(Message {
            task: strong,
            index: 0,
            count: parallelism.get(),
            error: send,
        });
        while let Some(inner) = outer.take() {
            outer = inner.next();
            self.send.send(inner).expect(
                "`Sender<T>` and `Receiver<T>` can't be disconnected as long as `Self` lives",
            );
        }
        (weak, receive)
    }

    fn next(&self) -> Option<usize> {
        let mut start = self.size.start.load(Ordering::Relaxed);
        loop {
            let end = self.size.end.load(Ordering::Relaxed);
            let next = start.checked_add(1)?.min(end);
            start = match self.size.start.compare_exchange_weak(
                start,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(next) if next < end => break Some(next),
                Ok(_) => break None,
                Err(start) => start,
            }
        }
    }

    fn spawn(strong: &Arc<Self>, index: usize) -> usize {
        // Use a weak reference to allow the pool to drop the state if there is no more
        // user-side reference to it.
        let weak = Arc::downgrade(strong);
        spawn(move || Self::run(weak, index));
        strong.ready.fetch_add(1, Ordering::Relaxed)
    }

    fn run(state: Weak<Self>, index: usize) {
        while let Some(state) = state.upgrade() {
            if state.size.end.load(Ordering::Relaxed) < index {
                // The pool has shrunk.
                state.size.start.fetch_sub(1, Ordering::Relaxed);
                break;
            }
            let Ok(message) = state.receive.recv() else {
                // The pool has been dropped.
                break;
            };
            state.ready.fetch_sub(1, Ordering::Relaxed);
            let run = catch_unwind(|| message.task.run(message.index, message.count));
            let done = catch_unwind(|| message.task.done());
            match (run, done) {
                (Ok(_), Ok(_)) => {}
                (Err(error), _) | (_, Err(error)) => {
                    let _ = message.error.try_send(cast_ref(&error).map(String::from));
                    State::spawn(&state, index);
                    resume_unwind(error);
                }
            }
            state.ready.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl Task for () {
    fn run(&self, _: usize, _: usize) {}

    fn done(&self) -> bool {
        false
    }
}

impl<T: Task + ?Sized> Task for Arc<T> {
    fn run(&self, index: usize, count: usize) {
        T::run(self, index, count)
    }

    fn done(&self) -> bool {
        T::done(self)
    }
}

impl Token<'_> {
    pub const fn done(&self) -> bool {
        self.0
    }
}

impl Executor {
    fn new(state: Arc<State>, parallelism: Option<NonZeroUsize>) -> Self {
        Self {
            state,
            parallelism: self::parallelism(parallelism),
        }
    }

    pub fn with(self, parallelism: NonZeroUsize) -> Self {
        Self {
            state: self.state,
            parallelism: self::parallelism(Some(parallelism)),
        }
    }

    pub const fn parallelism(&self) -> NonZeroUsize {
        self.parallelism
    }

    /// By borrowing the pool for the duration of the iterator, the pool
    /// can't get dropped while the iterator lives. Since the
    /// iterator may keep some of threads alive, it prevents threads
    /// from lingering after the pool is dropped.
    pub fn iterate<'a, T: Send + 'a, N: Fn(Yield<T>) -> Token + Send + Sync + 'a>(
        &self,
        next: N,
    ) -> Iterator<'a, T> {
        self.iterate_with(|_, _| {}, move |_, yields| next(yields))
    }

    pub fn iterate_with<
        'a,
        S,
        W: Fn(usize, usize) -> S + Send + Sync + 'a,
        T: Send + 'a,
        N: for<'b> Fn(&mut S, Yield<'b, T>) -> Token<'b> + Send + Sync + 'a,
    >(
        &self,
        with: W,
        next: N,
    ) -> Iterator<'a, T> {
        struct Inner<T, W, N>(W, N, Sender<Item<T>>);
        struct Outer<T, W, N>(RwLock<Option<Inner<T, W, N>>>);

        impl<S, W: Fn(usize, usize) -> S, T, N: for<'b> Fn(&mut S, Yield<'b, T>) -> Token<'b>> Task
            for Outer<T, W, N>
        {
            fn run(&self, index: usize, count: usize) {
                let mut state = if let Ok(Some(Inner(with, _, _))) = self.0.try_read().as_deref() {
                    with(index, count)
                } else {
                    return;
                };
                while let Ok(Some(Inner(_, next, send))) = self.0.try_read().as_deref() {
                    if next(&mut state, Yield(send)).0 {
                        break;
                    }
                }
            }

            fn done(&self) -> bool {
                match self.0.write().as_deref_mut() {
                    Ok(value) => value.take().is_some(),
                    Err(error) => error.get_mut().take().is_some(),
                }
            }
        }

        let (send, receive) = unbounded();
        let state = Arc::new(Outer(RwLock::new(Some(Inner(with, next, send)))));
        // SAFETY: The lifetimes of `W`, `T` and `N` are tracked by `Iterator` and the
        // `Task` that owns them is guaranteed to be dropped before the lifetime `'a`
        // ends.
        let task = unsafe {
            // Used the same lifetime extension trick as used in `std::thread::scope`.
            Arc::from_raw(Arc::into_raw(state) as *const (dyn Task + Send + Sync + 'static))
        };
        let (task, error) = self.state.send(task, self.parallelism);
        Iterator {
            task,
            item: Some(receive),
            error,
        }
    }
}

impl<'a, T> Yield<'a, T> {
    pub const fn skip(self) -> Token<'a> {
        Token(false, PhantomData)
    }

    pub fn next(self, item: T) -> Token<'a> {
        Token(self.0.send(Item::Next(item)).is_err(), PhantomData)
    }

    pub fn last(self, item: T) -> Token<'a> {
        let _ = self.0.send(Item::Last(item));
        Token(true, PhantomData)
    }

    pub fn done(self) -> Token<'a> {
        let _ = self.0.send(Item::Done).is_ok();
        Token(true, PhantomData)
    }
}

impl<T> Iterator<'_, T> {
    fn done(&self) -> bool {
        self.item.is_none()
    }
}

impl<T> iter::Iterator for Iterator<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(error) = self.error.try_recv() {
            let error = error.as_ref().map_or("a thread panicked", String::as_str);
            panic!("{error}");
        }

        match self.item.as_mut()?.recv().ok()? {
            Item::Next(value) => Some(value),
            Item::Last(value) => {
                self.item.take();
                Some(value)
            }
            Item::Done => {
                self.item.take();
                None
            }
        }
    }
}

impl<T> iter::FusedIterator for Iterator<'_, T> {}

impl<T> Drop for Iterator<'_, T> {
    fn drop(&mut self) {
        self.item.take();
        if let Some(task) = self.task.upgrade() {
            task.done();
        }
    }
}

/// Returns an [`Iterator<T>`] that will yield its values in parallel based of
/// the provided closure `N`. `N` will be called by many threads in the global
/// thread pool and accumulated in the iterator asynchronously.
///
/// On each call to `N`, a single [`Option<T>`] can yielded through the
/// [`Yield<T>`] object. If it is a [`Some<T>`], the item `T` will be channeled
/// to the [`Iterator<T>`]. If it is a [`None`], the [`Iterator<T>`] will stop
/// yielding items and all threads will stop calling `N`.
///
/// When calling `next` on the [`Iterator<T>`], if an item `T` is available, it
/// will be yielded, otherwise execution will block until an item is available
/// or `None` has been yielded.
pub fn iterate<'a, T: Send + 'a, N: Fn(Yield<T>) -> Token + Send + Sync + 'a>(
    next: N,
) -> Iterator<'a, T> {
    Pool::global().executor(None).iterate(next)
}

pub fn iterate_with<
    'a,
    S,
    W: Fn(usize, usize) -> S + Send + Sync + 'a,
    T: Send + 'a,
    N: for<'b> Fn(&mut S, Yield<'b, T>) -> Token<'b> + Send + Sync + 'a,
>(
    with: W,
    next: N,
) -> Iterator<'a, T> {
    Pool::global().executor(None).iterate_with(with, next)
}

fn size(size: Option<usize>) -> usize {
    match size {
        Some(size) => size,
        None => available_parallelism().map_or(8, NonZeroUsize::get) * 2,
    }
}

fn parallelism(parallelism: Option<NonZeroUsize>) -> NonZeroUsize {
    match parallelism {
        Some(parallelism) => parallelism,
        None => available_parallelism().unwrap_or(NonZeroUsize::MIN),
    }
}
