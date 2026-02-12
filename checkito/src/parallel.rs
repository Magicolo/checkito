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
