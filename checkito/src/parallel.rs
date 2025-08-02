use core::{
    iter,
    marker::PhantomData,
    mem::{forget, replace},
    num::NonZeroUsize,
    ops::Range,
    sync::atomic::{AtomicUsize, Ordering},
};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::{
    sync::{Arc, OnceLock, RwLock, TryLockError, Weak},
    thread::{available_parallelism, spawn},
};

pub struct Iterator<'a, T> {
    task: Weak<dyn Task + Send + Sync + 'a>,
    receive: Option<Receiver<Message<T>>>,
}

/// A yield object which *can* and *must* be used exactly once per call of its
/// iterator closure and will produce a [`Token<T>`] as an outcome. It is the
/// *only* way to produce a [`Token<T>`].
///
/// The reason for this pattern instead of simply returning a value from the
/// iterator closure is to allow synchronization to happen around yielding
/// items (ex: using a lock), which isn't feasible with a return pattern. This
/// is particularly useful to guarantee some ordering of the iterator values.
pub struct Yield<'a, T>(&'a Sender<Message<T>>);

/// A token given as an outcome of a [`Yield<T>`] operation. It is meant to
/// ensure that [`Yield<T>`] is called exactly once per call of the iterator.
///
/// The lifetime `'a` survives in the token to prevent exchanging tokens between
/// iterators.
pub struct Token<'a>(bool, PhantomData<&'a ()>);

#[derive(Clone)]
pub struct Pool(Arc<State>);

pub struct Executor {
    state: Arc<State>,
    parallelism: NonZeroUsize,
}

struct State {
    send: Sender<(StrongTask, usize, NonZeroUsize)>,
    receive: Receiver<(StrongTask, usize, NonZeroUsize)>,
    ready: AtomicUsize,
    size: Range<AtomicUsize>,
}

enum Message<T> {
    Next(T),
    Last(T),
    Done,
}

type StrongTask = Arc<dyn Task + Send + Sync + 'static>;
type WeakTask = Weak<dyn Task + Send + Sync + 'static>;

trait Task {
    /// Will be called once per thread in the pool providing the `index` of the
    /// thread.
    fn run(&self, index: usize, count: usize);
    fn done(&self) -> bool;
}

impl Pool {
    pub fn global() -> &'static Self {
        static POOL: OnceLock<Pool> = OnceLock::new();
        POOL.get_or_init(|| Pool::new(None))
    }

    pub fn new(size: Option<usize>) -> Self {
        Self(Arc::new(State::new(size)))
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

    fn send(self: &Arc<Self>, strong: StrongTask, parallelism: NonZeroUsize) -> Option<WeakTask> {
        let weak = Arc::downgrade(&strong);
        let mut ready = self.ready.load(Ordering::Relaxed);
        while ready < parallelism.get() {
            match self.next() {
                Some(index) => ready = Self::spawn(self, index),
                None => break,
            }
        }
        self.send.send((strong, 0, parallelism)).ok()?;
        // A weak reference is returned to allow the threads to drop the task as soon as
        // it's done.
        Some(weak)
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

    fn spawn(state: &Arc<Self>, index: usize) -> usize {
        let weak = Arc::downgrade(state);
        spawn(move || Self::run(weak, index));
        state.ready.fetch_add(1, Ordering::Relaxed)
    }

    fn run(state: Weak<Self>, index: usize) {
        struct DoneOnDrop(Arc<dyn Task>);
        struct SpawnOnDrop(Weak<State>, usize);

        impl Drop for DoneOnDrop {
            fn drop(&mut self) {
                // `Task::done` must be called even in the event of a panic.
                self.0.done();
            }
        }

        impl Drop for SpawnOnDrop {
            fn drop(&mut self) {
                if let Some(state) = replace(&mut self.0, Weak::new()).upgrade() {
                    State::spawn(&state, self.1);
                }
            }
        }

        let sentinel = SpawnOnDrop(state, index);
        while let Some(state) = sentinel.0.upgrade() {
            if state.size.end.load(Ordering::Relaxed) < sentinel.1 {
                // The pool has shrunk.
                state.size.start.fetch_sub(1, Ordering::Relaxed);
                break;
            }
            let Ok((task, index, count)) = state.receive.recv() else {
                // The pool has been dropped.
                break;
            };
            state.ready.fetch_sub(1, Ordering::Relaxed);
            let next = index + 1;
            if next < count.get() {
                state.send.send((task.clone(), next, count)).ok();
            }
            DoneOnDrop(task).0.run(index, count.get());
            state.ready.fetch_add(1, Ordering::Relaxed);
        }
        forget(sentinel);
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
        struct State<T, W, N>(RwLock<Option<(W, N, Sender<Message<T>>)>>);

        impl<S, W: Fn(usize, usize) -> S, T, N: for<'b> Fn(&mut S, Yield<'b, T>) -> Token<'b>> Task
            for State<T, W, N>
        {
            fn run(&self, index: usize, count: usize) {
                let mut state = if let Ok(Some((with, _, _))) = self.0.try_read().as_deref() {
                    with(index, count)
                } else {
                    return;
                };
                while let Ok(Some((_, next, send))) = self.0.try_read().as_deref() {
                    if next(&mut state, Yield(send)).0 {
                        break;
                    }
                }
            }

            fn done(&self) -> bool {
                take(&self.0).is_some()
            }
        }

        let (send, receive) = unbounded();
        let state = Arc::new(State(RwLock::new(Some((with, next, send)))));
        // SAFETY: The lifetimes of `W`, `T` and `N` are tracked by `Iterator` and the
        // `Task` that owns them is guaranteed to be dropped before the lifetime `'a`
        // ends.
        let task = unsafe {
            // Used the same lifetime extension trick as used in `std::thread::scope`.
            Arc::from_raw(Arc::into_raw(state) as *const (dyn Task + Send + Sync + 'static))
        };
        // TODO: Find a way to propagate the panic to this thread.
        let task = self
            .state
            .send(task, self.parallelism)
            .expect("a thread has panicked");
        Iterator {
            task,
            receive: Some(receive),
        }
    }
}

impl<'a, T> Yield<'a, T> {
    pub const fn skip(self) -> Token<'a> {
        Token(false, PhantomData)
    }

    pub fn next(self, item: T) -> Token<'a> {
        Token(self.0.send(Message::Next(item)).is_err(), PhantomData)
    }

    pub fn last(self, item: T) -> Token<'a> {
        let _ = self.0.send(Message::Last(item));
        Token(true, PhantomData)
    }

    pub fn done(self) -> Token<'a> {
        let _ = self.0.send(Message::Done).is_ok();
        Token(true, PhantomData)
    }
}

impl<T> Iterator<'_, T> {
    fn done(&self) -> bool {
        self.receive.is_none()
    }
}

impl<T> iter::Iterator for Iterator<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.receive.as_mut()?.recv().ok()? {
            Message::Next(value) => Some(value),
            Message::Last(value) => {
                self.receive.take();
                Some(value)
            }
            Message::Done => {
                self.receive.take();
                None
            }
        }
    }
}

impl<T> iter::FusedIterator for Iterator<'_, T> {}

impl<T> Drop for Iterator<'_, T> {
    fn drop(&mut self) {
        self.receive.take();
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

fn take<T>(lock: &RwLock<Option<T>>) -> Option<T> {
    match lock.write().as_deref_mut() {
        Ok(value) => value.take(),
        Err(error) => error.get_mut().take(),
    }
}

fn try_take<T>(lock: &RwLock<Option<T>>) -> Option<T> {
    match lock.try_write().as_deref_mut() {
        Ok(value) => value.take(),
        Err(TryLockError::Poisoned(error)) => error.get_mut().take(),
        Err(TryLockError::WouldBlock) => None,
    }
}

fn size(size: Option<usize>) -> usize {
    match size {
        Some(size) => size,
        None => available_parallelism().map_or(8, NonZeroUsize::get),
    }
}

fn parallelism(parallelism: Option<NonZeroUsize>) -> NonZeroUsize {
    match parallelism {
        Some(parallelism) => parallelism,
        None => available_parallelism().unwrap_or(NonZeroUsize::MIN),
    }
}
