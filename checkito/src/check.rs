use crate::{
    GENERATES, SHRINKS,
    generate::Generate,
    prove::Prove,
    shrink::Shrink,
    state::{self, Modes, Sizes, State, States},
};
use core::{
    fmt,
    future::Future,
    mem::replace,
    ops::{self, Deref, DerefMut},
    panic::AssertUnwindSafe,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll, ready},
};
use orn::Or3;
use std::{
    any::Any,
    borrow::Cow,
    error,
    panic::catch_unwind,
    result,
    sync::{Mutex, RwLock},
};

/// Bounds the generation process.
#[derive(Clone, Debug)]
pub struct Generates {
    /// Seed for the random number generator used to generate random primitives.
    ///
    /// Defaults to a random value.
    pub seed: u64,
    /// Range of sizes that will be gradually traversed while generating values.
    ///
    /// Defaults to `0.0..1.0`.
    pub sizes: Sizes,
    /// Maximum number of items that will be generated.
    ///
    /// Setting this to `0` will cause the [`Checks`] to do nothing.
    ///
    /// Defaults to [`GENERATES`].
    pub count: usize,
    /// Whether or not the [`Checks`] iterator will yield generation items.
    ///
    /// Defaults to `true`.
    pub items: bool,
    /// - `Some(true)` => Will generate all possible samples ignoring
    ///   [`Generates::seed`] and [`Generates::sizes`].
    /// - `Some(false)` => Will generate [`Generates::count`] random samples
    ///   using [`Generates::seed`] and [`Generates::sizes`].
    /// - `None` => Will determine exhaustiveness based on whether
    ///   [`Generate::cardinality`] is `<=` than [`Generates::count`].
    pub exhaustive: Option<bool>,
}

/// Bounds the shrinking process.
#[derive(Clone, Debug)]
pub struct Shrinks {
    /// Maximum number of attempts at shrinking an item that has failed a check.
    ///
    /// Setting this to `0` will disable shrinking.
    ///
    /// Defaults to [`SHRINKS`].
    pub count: usize,
    /// Whether or not the [`Checks`] iterator will yield shrinking items.
    ///
    /// Defaults to `true`.
    pub items: bool,
    /// Whether or not the [`Checks`] iterator will yield shrinking errors.
    ///
    /// Defaults to `true`.
    pub errors: bool,
}

/// The [`Checker`] structure holds a reference to a [`Generate`] instance and
/// some configuration options for the checking and shrinking processes.
#[derive(Debug, Clone)]
pub struct Checker<G: ?Sized, R> {
    /// Bounds the generation process.
    pub generate: Generates,
    /// Bounds the shrinking process.
    pub shrink: Shrinks,
    _run: R,
    /// A generator that will generate items and their shrinkers for checking a
    /// property.
    pub generator: G,
}

/// This structure is used to iterate over a sequence of check results.
/// - The iterator initially starts in a generate phase where it generates items
///   and it runs check against them.
/// - If a check passes, a [`Result::Pass`] is produced.
/// - If a check fails, the iterator enters the shrinking phase.
/// - When shrinking, the iterator tries to repeatedly shrink the previous item
///   and run checks against the shrunk items.
/// - It the check passes, a [`Result::Shrink`] is produced and it means that
///   the shrunk item has failed to reproduce a failing check.
/// - If the check fails, a [`Result::Shrunk`] is produced and it means that the
///   shrunk item has successfully reproduced a failing check and it becomes
///   current item.
/// - When the item is fully shrunk, the iterator produces a [`Result::Fail`]
///   with the final shrunk item in it.
///
/// This iterator guarantees to:
/// - Yield no results if [`Generates::count`] is set to `0`.
/// - Yield no results if [`Generates::items`] is set to `false` and all checks
///   passed.
/// - Yield only [`Result::Pass`] results if [`Generates::items`] is set to
///   `true` and all checks passed.
/// - Never yield a [`Result::Pass`] after a check has failed.
/// - Always yield a single final result of [`Result::Fail`] if at least a check
///   failed.
/// - Yield at most the smallest number between [`Generate::cardinality`] and
///   [`Generates::count`] [`Result::Pass`] results.
pub struct Checks<F, M> {
    yields: (bool, bool, bool),
    check: F,
    machine: M,
}

pub trait Check: Generate {
    fn checker(self) -> Checker<Self, synchronous::sequential::Run>
    where
        Self: Sized,
    {
        Checker::new(self, state::seed())
    }

    // TODO: Use the parallel implementation?
    fn checks<P: Prove, F: FnMut(Self::Item) -> P>(
        self,
        check: F,
    ) -> Checks<F, synchronous::sequential::Machine<Self, P>>
    where
        Self: Sized,
    {
        self.checker().checks(check)
    }

    // TODO: Use the parallel implementation?
    fn check<P: Prove, F: FnMut(Self::Item) -> P>(
        &self,
        check: F,
    ) -> Option<Fail<Self::Item, P::Error>> {
        self.checker().check(check)
    }
}

#[derive(Clone, Debug)]
pub enum Result<T, P: Prove> {
    /// An item was generated and passed the check.
    Pass(Pass<T, P::Proof>),
    /// An item was shrunk and passed the check, thus the shrinking is rejected.
    Shrink(Pass<T, P::Proof>),
    /// An item was shrunk and failed the check, thus the shrinking is accepted.
    Shrunk(Fail<T, P::Error>),
    /// The last generated of shrunk item that failed the check.
    Fail(Fail<T, P::Error>),
}

#[derive(Clone, Debug)]
/// A structure that represents a passed check.
pub struct Pass<T, P> {
    pub item: T,
    pub proof: P,
    pub generates: usize,
    pub shrinks: usize,
    /// The generator state that produced the item.
    pub state: State,
}

#[derive(Clone, Debug)]
/// A structure that represents a failed check.
pub struct Fail<T, E> {
    pub item: T,
    pub cause: Cause<E>,
    pub generates: usize,
    pub shrinks: usize,
    /// The generator state that caused the error.
    pub state: State,
}

/// The cause of a check failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cause<E> {
    /// A `Disprove` cause is a value that, when checked, returns a value of
    /// type `P` that does not satisfy the property.
    Disprove(E),
    /// A `Panic` cause is produced when a check panics during its evaluation.
    /// The message associated with the panic is included if it can be casted to
    /// a string.
    Panic(Option<Cow<'static, str>>),
}

impl<G: Generate + ?Sized> Check for G {}

impl<G: Generate> Checker<G, synchronous::sequential::Run> {
    pub(crate) const fn new(generator: G, seed: u64) -> Self {
        Self {
            generator,
            generate: Generates {
                items: true,
                count: GENERATES,
                seed,
                sizes: Sizes::DEFAULT,
                exhaustive: None,
            },
            shrink: Shrinks {
                count: SHRINKS,
                items: true,
                errors: true,
            },
            _run: synchronous::sequential::Run,
        }
    }
}

impl<G: Generate, R> Checker<G, R> {
    fn with<S>(self, run: S) -> Checker<G, S> {
        Checker {
            generate: self.generate,
            shrink: self.shrink,
            generator: self.generator,
            _run: run,
        }
    }
}

impl<T, P: Prove> Result<T, P> {
    pub const fn seed(&self) -> u64 {
        match self {
            Result::Pass(pass) | Result::Shrink(pass) => pass.seed(),
            Result::Fail(fail) | Result::Shrunk(fail) => fail.seed(),
        }
    }

    pub const fn size(&self) -> f64 {
        match self {
            Result::Pass(pass) | Result::Shrink(pass) => pass.size(),
            Result::Fail(fail) | Result::Shrunk(fail) => fail.size(),
        }
    }

    pub const fn generates(&self) -> usize {
        match self {
            Result::Pass(pass) | Result::Shrink(pass) => pass.generates,
            Result::Fail(fail) | Result::Shrunk(fail) => fail.generates,
        }
    }

    pub const fn shrinks(&self) -> usize {
        match self {
            Result::Pass(pass) | Result::Shrink(pass) => pass.shrinks,
            Result::Fail(fail) | Result::Shrunk(fail) => fail.shrinks,
        }
    }

    pub const fn state(&self) -> &State {
        match self {
            Result::Pass(pass) | Result::Shrink(pass) => &pass.state,
            Result::Fail(fail) | Result::Shrunk(fail) => &fail.state,
        }
    }

    pub fn pass(self, shrink: bool) -> Option<Pass<T, P::Proof>> {
        match self {
            Result::Pass(pass) => Some(pass),
            Result::Shrink(pass) if shrink => Some(pass),
            Result::Fail(_) | Result::Shrink(_) | Result::Shrunk(_) => None,
        }
    }

    pub fn fail(self, shrunk: bool) -> Option<Fail<T, P::Error>> {
        match self {
            Result::Fail(fail) => Some(fail),
            Result::Shrunk(fail) if shrunk => Some(fail),
            Result::Pass(_) | Result::Shrunk(_) | Result::Shrink(_) => None,
        }
    }

    pub fn item(self) -> T {
        match self {
            Result::Pass(pass) | Result::Shrink(pass) => pass.item,
            Result::Fail(fail) | Result::Shrunk(fail) => fail.item,
        }
    }

    #[allow(clippy::result_large_err)]
    pub fn result(self) -> result::Result<Pass<T, P::Proof>, Fail<T, P::Error>> {
        match self {
            Result::Pass(pass) | Result::Shrink(pass) => Ok(pass),
            Result::Fail(fail) | Result::Shrunk(fail) => Err(fail),
        }
    }
}

impl<T, P> Pass<T, P> {
    pub const fn seed(&self) -> u64 {
        self.state.seed()
    }

    pub const fn size(&self) -> f64 {
        self.state.size()
    }
}

impl<T, P> Fail<T, P> {
    pub const fn seed(&self) -> u64 {
        self.state.seed()
    }

    pub const fn size(&self) -> f64 {
        self.state.size()
    }

    pub fn message(&self) -> Cow<'static, str>
    where
        P: fmt::Debug,
    {
        match &self.cause {
            Cause::Panic(Some(message)) => message.clone(),
            Cause::Panic(None) => "panicked".into(),
            Cause::Disprove(proof) => format!("{proof:?}").into(),
        }
    }
}

impl<T, P: Prove> Deref for Result<T, P> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Result::Pass(pass) | Result::Shrink(pass) => &pass.item,
            Result::Fail(fail) | Result::Shrunk(fail) => &fail.item,
        }
    }
}

impl<T, P: Prove> DerefMut for Result<T, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Result::Pass(pass) | Result::Shrink(pass) => &mut pass.item,
            Result::Fail(fail) | Result::Shrunk(fail) => &mut fail.item,
        }
    }
}

impl<T, P: Prove> AsRef<T> for Result<T, P> {
    fn as_ref(&self) -> &T {
        match self {
            Result::Pass(pass) | Result::Shrink(pass) => &pass.item,
            Result::Fail(fail) | Result::Shrunk(fail) => &fail.item,
        }
    }
}

impl<T, P: Prove> AsMut<T> for Result<T, P> {
    fn as_mut(&mut self) -> &mut T {
        match self {
            Result::Pass(pass) | Result::Shrink(pass) => &mut pass.item,
            Result::Fail(fail) | Result::Shrunk(fail) => &mut fail.item,
        }
    }
}

impl<T, P> Deref for Pass<T, P> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl<T, P> DerefMut for Pass<T, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}

impl<T, P> AsRef<T> for Pass<T, P> {
    fn as_ref(&self) -> &T {
        &self.item
    }
}

impl<T, P> AsMut<T> for Pass<T, P> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.item
    }
}

impl<T, P> Deref for Fail<T, P> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl<T, P> DerefMut for Fail<T, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}

impl<T, P> AsRef<T> for Fail<T, P> {
    fn as_ref(&self) -> &T {
        &self.item
    }
}

impl<T, P> AsMut<T> for Fail<T, P> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.item
    }
}

const fn pass<T, P: Prove>(item: T, state: State, proof: P::Proof) -> Result<T, P> {
    Result::Pass(Pass {
        item,
        generates: state.index() + 1,
        shrinks: 0,
        proof,
        state,
    })
}

const fn fail<T, P: Prove>(
    item: T,
    index: usize,
    state: State,
    cause: Cause<P::Error>,
) -> Result<T, P> {
    Result::Fail(Fail {
        item,
        generates: state.index() + 1,
        shrinks: index,
        state,
        cause,
    })
}

const fn shrink<T, P: Prove>(item: T, index: usize, state: State, proof: P::Proof) -> Result<T, P> {
    Result::Shrink(Pass {
        item,
        generates: state.index() + 1,
        shrinks: index,
        proof,
        state,
    })
}

const fn shrunk<T, P: Prove>(
    item: T,
    index: usize,
    state: State,
    cause: Cause<P::Error>,
) -> Result<T, P> {
    Result::Shrunk(Fail {
        item,
        generates: state.index() + 1,
        shrinks: index,
        state,
        cause,
    })
}

fn cast(error: Box<dyn Any + Send>) -> Option<Cow<'static, str>> {
    let error = match error.downcast::<&'static str>() {
        Ok(error) => return Some(Cow::Borrowed(*error)),
        Err(error) => error,
    };
    let error = match error.downcast::<String>() {
        Ok(error) => return Some(Cow::Owned(*error)),
        Err(error) => error,
    };
    let error = match error.downcast::<Box<str>>() {
        Ok(error) => return Some(Cow::Owned(error.to_string())),
        Err(error) => error,
    };
    match error.downcast::<Cow<'static, str>>() {
        Ok(error) => Some(*error),
        Err(_) => None,
    }
}

impl<T: fmt::Debug, E: fmt::Debug> fmt::Display for Fail<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<T: fmt::Debug, E: fmt::Debug> error::Error for Fail<T, E> {}

pub(crate) mod synchronous {
    use super::*;

    pub(crate) mod sequential {
        use super::*;

        pub struct Run;

        pub enum Machine<G: Generate, P: Prove> {
            Generate {
                generator: G,
                states: States,
                shrinks: ops::Range<usize>,
            },
            Shrink {
                index: usize,
                state: State,
                shrinks: ops::Range<usize>,
                shrinker: G::Shrink,
                cause: Cause<P::Error>,
            },
            Done,
        }

        impl<G: Generate> Checker<G, Run> {
            #[cfg(feature = "parallel")]
            pub fn parallel(self) -> Checker<G, parallel::Run>
            where
                G: Generate<Item: Send, Shrink: Send> + Send + Sync,
            {
                self.with(parallel::Run)
            }

            #[cfg(feature = "asynchronous")]
            pub fn asynchronous(self) -> Checker<G, asynchronous::sequential::Run>
            where
                G: Generate<Shrink: Unpin> + Unpin,
            {
                self.with(asynchronous::sequential::Run)
            }

            pub fn check<P: Prove, F: FnMut(G::Item) -> P>(
                mut self,
                check: F,
            ) -> Option<Fail<G::Item, P::Error>> {
                self.generate.items = false;
                self.shrink.items = false;
                self.shrink.errors = false;
                self.checks(check).last()?.fail(false)
            }

            pub fn checks<P: Prove, F: FnMut(G::Item) -> P>(
                self,
                check: F,
            ) -> Checks<F, Machine<G, P>> {
                let modes = Modes::with(
                    self.generate.count,
                    self.generate.sizes,
                    self.generate.seed,
                    self.generator.cardinality(),
                    self.generate.exhaustive,
                );
                Checks {
                    yields: (self.generate.items, self.shrink.items, self.shrink.errors),
                    machine: Machine::Generate {
                        generator: self.generator,
                        states: modes.into(),
                        shrinks: 0..self.shrink.count,
                    },
                    check,
                }
            }
        }

        impl<G: Generate, P: Prove, F: FnMut(G::Item) -> P> Iterator for Checks<F, Machine<G, P>> {
            type Item = Result<G::Item, P>;

            fn next(&mut self) -> Option<Self::Item> {
                loop {
                    match replace(&mut self.machine, Machine::Done) {
                        Machine::Generate {
                            generator,
                            mut states,
                            shrinks,
                        } => {
                            let mut state = states.next()?;
                            let shrinker = generator.generate(&mut state);
                            match handle(shrinker.item(), &mut self.check) {
                                Ok(proof) => {
                                    self.machine = Machine::Generate {
                                        generator,
                                        states,
                                        shrinks,
                                    };
                                    if self.yields.0 {
                                        break Some(pass(shrinker.item(), state, proof));
                                    }
                                }
                                Err(cause) => {
                                    self.machine = Machine::Shrink {
                                        index: 0,
                                        state,
                                        shrinker,
                                        shrinks,
                                        cause,
                                    };
                                }
                            }
                        }
                        Machine::Shrink {
                            index,
                            state,
                            mut shrinks,
                            shrinker: mut old_shrinker,
                            cause: old_cause,
                        } => {
                            let next = match shrinks.next() {
                                Some(index) => index,
                                None => {
                                    self.machine = Machine::Done;
                                    break Some(fail(old_shrinker.item(), index, state, old_cause));
                                }
                            };
                            let new_shrinker = match old_shrinker.shrink() {
                                Some(shrinker) => shrinker,
                                None => {
                                    self.machine = Machine::Done;
                                    break Some(fail(old_shrinker.item(), index, state, old_cause));
                                }
                            };
                            match handle(new_shrinker.item(), &mut self.check) {
                                Ok(proof) => {
                                    self.machine = Machine::Shrink {
                                        index: next,
                                        state: state.clone(),
                                        shrinks,
                                        shrinker: old_shrinker,
                                        cause: old_cause,
                                    };
                                    if self.yields.1 {
                                        break Some(shrink(
                                            new_shrinker.item(),
                                            next,
                                            state,
                                            proof,
                                        ));
                                    }
                                }
                                Err(new_cause) => {
                                    self.machine = Machine::Shrink {
                                        index: next,
                                        state: state.clone(),
                                        shrinks,
                                        shrinker: new_shrinker,
                                        cause: new_cause,
                                    };
                                    if self.yields.2 {
                                        break Some(shrunk(
                                            old_shrinker.item(),
                                            next,
                                            state,
                                            old_cause,
                                        ));
                                    }
                                }
                            }
                        }
                        Machine::Done => break None,
                    }
                }
            }
        }
    }

    #[cfg(feature = "parallel")]
    pub(crate) mod parallel {
        use super::*;
        use crate::parallel::iterate;
        use orn::Or2;
        use rayon::iter::{
            IntoParallelIterator, ParallelIterator, empty, once, plumbing::UnindexedConsumer,
        };

        pub struct Run;

        pub struct Machine<G: Generate> {
            generator: G,
            states: States,
            shrinks: ops::Range<usize>,
        }

        impl<G: Generate<Item: Send, Shrink: Send> + Send + Sync> Checker<G, Run> {
            pub fn sequential(self) -> Checker<G, sequential::Run> {
                self.with(sequential::Run)
            }

            #[cfg(feature = "asynchronous")]
            pub fn asynchronous(self) -> Checker<G, asynchronous::parallel::Run>
            where
                G: Generate<Shrink: Unpin> + Unpin,
            {
                self.with(asynchronous::parallel::Run)
            }

            pub fn check<P: Prove<Proof: Send, Error: Send>, F: Fn(G::Item) -> P + Send + Sync>(
                mut self,
                check: F,
            ) -> Option<Fail<G::Item, P::Error>> {
                self.generate.items = false;
                self.shrink.items = false;
                self.shrink.errors = false;
                self.checks(check)
                    .find_last(|result| matches!(result, Result::Fail(..)))?
                    .fail(false)
            }

            pub fn checkz<
                'a,
                P: Prove<Proof: Send, Error: Send> + 'a,
                F: Fn(G::Item) -> P + Send + Sync + 'a,
            >(
                self,
                check: F,
            ) -> crate::parallel::Iterator<'a, Result<G::Item, P>>
            where
                G: 'a,
            {
                enum Machine<G> {
                    Generate {
                        generator: G,
                        modes: Modes,
                        shrinks: ops::Range<usize>,
                    },
                }
                let modes = Modes::with(
                    self.generate.count,
                    self.generate.sizes,
                    self.generate.seed,
                    self.generator.cardinality(),
                    self.generate.exhaustive,
                );
                let index = AtomicUsize::new(0);
                iterate(move |yields| {
                    let index = index.fetch_add(1, Ordering::Relaxed);
                    let Some(mut state) = modes.state(index) else {
                        return yields.done();
                    };
                    let shrinker = self.generator.generate(&mut state);
                    match handle(shrinker.item(), &check) {
                        Ok(proof) => yields.next(pass(shrinker.item(), state, proof)),
                        Err(cause) => yields.last(fail(shrinker.item(), 0, state, cause)),
                    }
                })
            }

            pub fn checks<P: Prove<Proof: Send, Error: Send>, F: Fn(G::Item) -> P + Send + Sync>(
                self,
                check: F,
            ) -> Checks<F, Machine<G>> {
                let modes = Modes::with(
                    self.generate.count,
                    self.generate.sizes,
                    self.generate.seed,
                    self.generator.cardinality(),
                    self.generate.exhaustive,
                );
                Checks {
                    yields: (self.generate.items, self.shrink.items, self.shrink.errors),
                    machine: Machine {
                        generator: self.generator,
                        states: modes.into(),
                        shrinks: 0..self.shrink.count,
                    },
                    check,
                }
            }
        }

        impl<
            G: Generate<Item: Send, Shrink: Send> + Send + Sync,
            P: Prove<Proof: Send, Error: Send>,
            F: Fn(G::Item) -> P + Send + Sync,
        > ParallelIterator for Checks<F, Machine<G>>
        {
            type Item = Result<G::Item, P>;

            fn drive_unindexed<C>(self, consumer: C) -> C::Result
            where
                C: UnindexedConsumer<Self::Item>,
            {
                let Self {
                    yields,
                    check,
                    machine,
                } = self;
                let some = |value| once::<Option<Self::Item>>(Some(value));
                let none = || once::<Option<Self::Item>>(None);
                let empty = empty::<Option<Self::Item>>;
                let check = RwLock::new(Some(check));
                machine
                    .states
                    .into_par_iter()
                    .flat_map(move |mut state| {
                        let shrinker = machine.generator.generate(&mut state);
                        let result = {
                            let Ok(guard) = check.try_read() else {
                                return Or3::T0(none());
                            };
                            let Some(guard) = guard.as_ref() else {
                                return Or3::T0(none());
                            };
                            handle(shrinker.item(), guard)
                        };
                        match result {
                            Ok(proof) => {
                                if yields.0 {
                                    Or3::T0(some(pass(shrinker.item(), state, proof)))
                                } else {
                                    Or3::T1(empty())
                                }
                            }
                            Err(cause) => {
                                let check = {
                                    let Ok(mut guard) = check.write() else {
                                        return Or3::T0(none());
                                    };
                                    let Some(check) = guard.take() else {
                                        return Or3::T0(none());
                                    };
                                    check
                                };
                                let pair = Mutex::new(Some((shrinker, cause)));
                                let count = AtomicUsize::new(0);
                                Or3::T2(machine.shrinks.clone().into_par_iter().flat_map(
                                    move |_| {
                                        let index = count.fetch_add(1, Ordering::Relaxed);
                                        let new_shrinker = {
                                            let Ok(mut guard) = pair.lock() else {
                                                return Or2::T0(none());
                                            };
                                            let Some((mut old_shrinker, old_cause)) = guard.take()
                                            else {
                                                return Or2::T0(none());
                                            };
                                            match old_shrinker.shrink() {
                                                Some(new_shrinker) => {
                                                    *guard = Some((old_shrinker, old_cause));
                                                    new_shrinker
                                                }
                                                None => {
                                                    return Or2::T0(some(fail(
                                                        old_shrinker.item(),
                                                        index,
                                                        state.clone(),
                                                        old_cause,
                                                    )));
                                                }
                                            }
                                        };

                                        match handle(new_shrinker.item(), &check) {
                                            Ok(new_proof) => {
                                                if yields.1 {
                                                    Or2::T0(some(shrink(
                                                        new_shrinker.item(),
                                                        index + 1,
                                                        state.clone(),
                                                        new_proof,
                                                    )))
                                                } else {
                                                    Or2::T1(empty())
                                                }
                                            }
                                            Err(new_cause) => {
                                                let Ok(mut guard) = pair.lock() else {
                                                    return Or2::T0(none());
                                                };
                                                let Some(pair) = guard.as_mut() else {
                                                    return Or2::T0(none());
                                                };
                                                let (old_shrinker, old_cause) =
                                                    replace(pair, (new_shrinker, new_cause));

                                                if yields.2 {
                                                    Or2::T0(some(shrunk(
                                                        old_shrinker.item(),
                                                        index + 1,
                                                        state.clone(),
                                                        old_cause,
                                                    )))
                                                } else {
                                                    Or2::T1(empty())
                                                }
                                            }
                                        }
                                    },
                                ))
                            }
                        }
                    })
                    .map(|or| match or {
                        Or3::T0(value) | Or3::T1(value) => value,
                        Or3::T2(Or2::T0(value) | Or2::T1(value)) => value,
                    })
                    .while_some()
                    .drive_unindexed(consumer)
            }
        }
    }

    fn handle<T, P: Prove, F: FnMut(T) -> P>(
        item: T,
        mut check: F,
    ) -> result::Result<P::Proof, Cause<P::Error>> {
        match catch_unwind(AssertUnwindSafe(move || check(item))) {
            Ok(prove) => match prove.prove() {
                Ok(ok) => Ok(ok),
                Err(error) => Err(Cause::Disprove(error)),
            },
            Err(error) => Err(Cause::Panic(cast(error))),
        }
    }
}

#[cfg(feature = "asynchronous")]
pub(crate) mod asynchronous {
    use super::*;

    pub(crate) mod sequential {
        use super::*;
        use futures_lite::{Stream, StreamExt};

        pub struct Run;

        pub enum Machine<G: Generate, P: Future<Output: Prove>> {
            Generate {
                generator: G,
                states: States,
                shrinks: ops::Range<usize>,
                pin: Option<Pin<Box<P>>>,
            },
            Handle1 {
                generator: G,
                states: States,
                state: State,
                shrinks: ops::Range<usize>,
                shrinker: G::Shrink,
                pin: Pin<Box<P>>,
            },
            Shrink {
                index: usize,
                state: State,
                shrinks: ops::Range<usize>,
                shrinker: G::Shrink,
                cause: Cause<<P::Output as Prove>::Error>,
                pin: Option<Pin<Box<P>>>,
            },
            Handle2 {
                index: usize,
                state: State,
                shrinks: ops::Range<usize>,
                old: G::Shrink,
                new: G::Shrink,
                cause: Cause<<P::Output as Prove>::Error>,
                pin: Pin<Box<P>>,
            },
            Done,
        }

        impl<G: Generate<Shrink: Unpin> + Unpin> Checker<G, Run> {
            #[cfg(feature = "parallel")]
            pub fn parallel(self) -> Checker<G, asynchronous::parallel::Run>
            where
                G: Generate<Item: Send, Shrink: Send> + Send + Sync,
            {
                self.with(asynchronous::parallel::Run)
            }

            pub fn synchronous(self) -> Checker<G, synchronous::sequential::Run> {
                self.with(synchronous::sequential::Run)
            }

            pub async fn check<
                P: Future<Output: Prove<Error: Unpin> + Unpin>,
                F: FnMut(G::Item) -> P + Unpin,
            >(
                mut self,
                check: F,
            ) -> Option<Fail<G::Item, <P::Output as Prove>::Error>> {
                self.generate.items = false;
                self.shrink.items = false;
                self.shrink.errors = false;
                self.checks(check).last().await?.fail(false)
            }

            pub fn checks<
                P: Future<Output: Prove<Error: Unpin> + Unpin>,
                F: FnMut(G::Item) -> P + Unpin,
            >(
                self,
                check: F,
            ) -> Checks<F, Machine<G, P>> {
                let modes = Modes::with(
                    self.generate.count,
                    self.generate.sizes,
                    self.generate.seed,
                    self.generator.cardinality(),
                    self.generate.exhaustive,
                );
                Checks {
                    yields: (self.generate.items, self.shrink.items, self.shrink.errors),
                    machine: Machine::Generate {
                        generator: self.generator,
                        shrinks: 0..self.shrink.count,
                        states: modes.into(),
                        pin: None,
                    },
                    check,
                }
            }
        }

        impl<
            G: Generate<Shrink: Unpin> + Unpin,
            P: Future<Output: Prove<Error: Unpin> + Unpin>,
            F: FnMut(G::Item) -> P + Unpin,
        > Stream for Checks<F, Machine<G, P>>
        {
            type Item = Result<G::Item, P::Output>;

            fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                let checks = Pin::into_inner(self);
                loop {
                    match replace(&mut checks.machine, Machine::Done) {
                        Machine::Generate {
                            generator,
                            mut states,
                            shrinks,
                            mut pin,
                        } => {
                            let Some(mut state) = states.next() else {
                                break Poll::Ready(None);
                            };
                            let shrinker = generator.generate(&mut state);
                            match prepare(shrinker.item(), &mut checks.check, &mut pin) {
                                Ok(pin) => {
                                    checks.machine = Machine::Handle1 {
                                        generator,
                                        states,
                                        state,
                                        shrinks,
                                        shrinker,
                                        pin,
                                    }
                                }
                                Err(cause) => {
                                    checks.machine = Machine::Shrink {
                                        index: 0,
                                        state,
                                        shrinks,
                                        shrinker,
                                        cause,
                                        pin,
                                    }
                                }
                            };
                        }
                        Machine::Handle1 {
                            generator,
                            states,
                            state,
                            shrinks,
                            shrinker,
                            mut pin,
                        } => match ready!(handle(pin.as_mut(), cx)) {
                            Ok(proof) => {
                                checks.machine = Machine::Generate {
                                    generator,
                                    states,
                                    shrinks,
                                    pin: Some(pin),
                                };
                                if checks.yields.0 {
                                    break Poll::Ready(Some(pass(shrinker.item(), state, proof)));
                                }
                            }
                            Err(cause) => {
                                checks.machine = Machine::Shrink {
                                    index: 0,
                                    state,
                                    shrinks,
                                    shrinker,
                                    cause,
                                    pin: Some(pin),
                                };
                            }
                        },
                        Machine::Shrink {
                            index,
                            state,
                            mut shrinks,
                            shrinker: mut old_shrinker,
                            cause: old_cause,
                            mut pin,
                        } => {
                            let next = match shrinks.next() {
                                Some(index) => index,
                                None => {
                                    checks.machine = Machine::Done;
                                    break Poll::Ready(Some(fail(
                                        old_shrinker.item(),
                                        index,
                                        state,
                                        old_cause,
                                    )));
                                }
                            };
                            let new_shrinker = match old_shrinker.shrink() {
                                Some(shrinker) => shrinker,
                                None => {
                                    checks.machine = Machine::Done;
                                    break Poll::Ready(Some(fail(
                                        old_shrinker.item(),
                                        index,
                                        state,
                                        old_cause,
                                    )));
                                }
                            };
                            match prepare(new_shrinker.item(), &mut checks.check, &mut pin) {
                                Ok(pin) => {
                                    checks.machine = Machine::Handle2 {
                                        index: next,
                                        state,
                                        old: old_shrinker,
                                        new: new_shrinker,
                                        shrinks,
                                        cause: old_cause,
                                        pin,
                                    }
                                }
                                Err(new_cause) => {
                                    checks.machine = Machine::Shrink {
                                        index: next,
                                        state: state.clone(),
                                        shrinks,
                                        shrinker: new_shrinker,
                                        cause: new_cause,
                                        pin,
                                    };
                                    if checks.yields.2 {
                                        break Poll::Ready(Some(shrunk(
                                            old_shrinker.item(),
                                            next,
                                            state,
                                            old_cause,
                                        )));
                                    }
                                }
                            }
                        }
                        Machine::Handle2 {
                            index,
                            state,
                            old,
                            new,
                            shrinks,
                            cause,
                            mut pin,
                        } => match ready!(handle(pin.as_mut(), cx)) {
                            Ok(proof) => {
                                checks.machine = Machine::Shrink {
                                    index,
                                    state: state.clone(),
                                    shrinks,
                                    shrinker: old,
                                    cause,
                                    pin: Some(pin),
                                };
                                if checks.yields.1 {
                                    break Poll::Ready(Some(shrink(
                                        new.item(),
                                        index,
                                        state,
                                        proof,
                                    )));
                                }
                            }
                            Err(new_cause) => {
                                checks.machine = Machine::Shrink {
                                    index,
                                    state: state.clone(),
                                    shrinks,
                                    shrinker: new,
                                    cause: new_cause,
                                    pin: Some(pin),
                                };
                                if checks.yields.2 {
                                    break Poll::Ready(Some(shrunk(
                                        old.item(),
                                        index,
                                        state,
                                        cause,
                                    )));
                                }
                            }
                        },
                        Machine::Done => break Poll::Ready(None),
                    }
                }
            }
        }

        fn prepare<T, P: Future<Output: Prove>, F: FnMut(T) -> P>(
            item: T,
            mut check: F,
            pin: &mut Option<Pin<Box<P>>>,
        ) -> result::Result<Pin<Box<P>>, Cause<<P::Output as Prove>::Error>> {
            match catch_unwind(AssertUnwindSafe(move || check(item))) {
                Ok(check) => Ok(match pin.take() {
                    Some(mut pin) => {
                        pin.set(check);
                        pin
                    }
                    None => Box::pin(check),
                }),
                Err(error) => Err(Cause::Panic(cast(error))),
            }
        }
    }

    #[cfg(feature = "parallel")]
    pub(crate) mod parallel {
        use super::*;

        pub struct Run;

        pub struct Machine<G: Generate> {
            generator: G,
            states: States,
            shrinks: ops::Range<usize>,
        }

        impl<G: Generate> Checker<G, Run> {
            pub fn sequential(self) -> Checker<G, asynchronous::sequential::Run> {
                self.with(asynchronous::sequential::Run)
            }

            pub fn synchronous(self) -> Checker<G, synchronous::parallel::Run> {
                self.with(synchronous::parallel::Run)
            }

            pub fn check<P: Future<Output: Prove>, F: Fn(G::Item) -> P>(
                mut self,
                check: F,
            ) -> Option<Fail<G::Item, <P::Output as Prove>::Error>> {
                self.generate.items = false;
                self.shrink.items = false;
                self.shrink.errors = false;
                todo!()
                // self.checks(check).last()?.fail(false)
            }

            pub fn checks<P: Future<Output: Prove>, F: Fn(G::Item) -> P>(
                self,
                check: F,
            ) -> Checks<F, Machine<G>> {
                let modes = Modes::with(
                    self.generate.count,
                    self.generate.sizes,
                    self.generate.seed,
                    self.generator.cardinality(),
                    self.generate.exhaustive,
                );
                Checks {
                    yields: (self.generate.items, self.shrink.items, self.shrink.errors),
                    machine: Machine {
                        generator: self.generator,
                        states: modes.into(),
                        shrinks: 0..self.shrink.count,
                    },
                    check,
                }
            }
        }
    }

    #[allow(clippy::type_complexity)]
    fn handle<P: Future<Output: Prove>>(
        check: Pin<&mut P>,
        context: &mut Context,
    ) -> Poll<result::Result<<P::Output as Prove>::Proof, Cause<<P::Output as Prove>::Error>>> {
        match catch_unwind(AssertUnwindSafe(move || check.poll(context))) {
            Ok(Poll::Pending) => Poll::Pending,
            Ok(Poll::Ready(prove)) => match prove.prove() {
                Ok(ok) => Poll::Ready(Ok(ok)),
                Err(error) => Poll::Ready(Err(Cause::Disprove(error))),
            },
            Err(error) => Poll::Ready(Err(Cause::Panic(cast(error)))),
        }
    }
}
