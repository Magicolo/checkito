use crate::{
    GENERATES, SHRINKS,
    generate::Generate,
    prove::Prove,
    shrink::Shrink,
    state::{self, Modes, Sizes, State, States},
};
use core::{
    fmt,
    marker::PhantomData,
    mem::replace,
    ops::{self, Deref, DerefMut},
    panic::AssertUnwindSafe,
};
use std::{any::Any, borrow::Cow, error, panic::catch_unwind, result};

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
pub struct Checker<G: ?Sized, R: ?Sized = synchronous::Run> {
    /// Bounds the generation process.
    pub generate: Generates,
    /// Bounds the shrinking process.
    pub shrink: Shrinks,
    _run: PhantomData<R>,
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
    shrinks: ops::Range<usize>,
    check: F,
    machine: M,
}

pub trait Check: Generate {
    fn checker(self) -> Checker<Self>
    where
        Self: Sized,
    {
        Checker::new(self, state::seed())
    }

    // TODO: Use the parallel implementation?
    fn checks<P: Prove, F: FnMut(Self::Item) -> P>(
        self,
        check: F,
    ) -> Checks<F, synchronous::Machine<Self, P>>
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

impl<G: Generate, R: ?Sized> Checker<G, R> {
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
            _run: PhantomData,
        }
    }

    pub fn synchronous(self) -> Checker<G, synchronous::Run> {
        self.map()
    }

    #[cfg(feature = "asynchronous")]
    pub fn asynchronous(self) -> Checker<G, asynchronous::Run> {
        self.map()
    }

    fn map<S>(self) -> Checker<G, S> {
        Checker {
            generate: self.generate,
            shrink: self.shrink,
            generator: self.generator,
            _run: PhantomData,
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

    pub struct Run;

    pub enum Machine<G: Generate, P: Prove> {
        Generate {
            generator: G,
            states: States,
        },
        Shrink {
            index: usize,
            state: State,
            shrinker: G::Shrink,
            cause: Cause<P::Error>,
        },
        Done,
    }

    impl<G: Generate> Checker<G, Run> {
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
                shrinks: 0..self.shrink.count,
                machine: Machine::Generate {
                    generator: self.generator,
                    states: modes.into(),
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
                    } => {
                        let mut state = states.next()?;
                        let shrinker = generator.generate(&mut state);
                        match handle(shrinker.item(), &mut self.check) {
                            Ok(proof) => {
                                self.machine = Machine::Generate { generator, states };
                                if self.yields.0 {
                                    break Some(pass(shrinker.item(), state, proof));
                                }
                            }
                            Err(cause) => {
                                self.machine = Machine::Shrink {
                                    index: 0,
                                    state,
                                    shrinker,
                                    cause,
                                };
                            }
                        }
                    }
                    Machine::Shrink {
                        index,
                        state,
                        mut shrinker,
                        cause,
                    } => {
                        let next = match self.shrinks.next() {
                            Some(index) => index,
                            None => {
                                self.machine = Machine::Done;
                                break Some(fail(shrinker.item(), index, state, cause));
                            }
                        };
                        let new = match shrinker.shrink() {
                            Some(shrinker) => shrinker,
                            None => {
                                self.machine = Machine::Done;
                                break Some(fail(shrinker.item(), index, state, cause));
                            }
                        };
                        match handle(new.item(), &mut self.check) {
                            Ok(proof) => {
                                self.machine = Machine::Shrink {
                                    index: next,
                                    state: state.clone(),
                                    shrinker,
                                    cause,
                                };
                                if self.yields.1 {
                                    break Some(shrink(new.item(), next, state, proof));
                                }
                            }
                            Err(new_cause) => {
                                self.machine = Machine::Shrink {
                                    index: next,
                                    state: state.clone(),
                                    shrinker: new,
                                    cause: new_cause,
                                };
                                if self.yields.2 {
                                    break Some(shrunk(shrinker.item(), next, state, cause));
                                }
                            }
                        }
                    }
                    Machine::Done => break None,
                }
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
    use core::{
        future::Future,
        pin::Pin,
        task::{Context, Poll, ready},
    };
    use futures_lite::{Stream, StreamExt};

    pub struct Run;

    pub enum Machine<G: Generate, P: Future<Output: Prove>> {
        Generate {
            generator: G,
            states: States,
            pin: Option<Pin<Box<P>>>,
        },
        Handle1 {
            generator: G,
            states: States,
            state: State,
            shrinker: G::Shrink,
            pin: Pin<Box<P>>,
        },
        Shrink {
            index: usize,
            state: State,
            shrinker: G::Shrink,
            cause: Cause<<P::Output as Prove>::Error>,
            pin: Option<Pin<Box<P>>>,
        },
        Handle2 {
            index: usize,
            state: State,
            old: G::Shrink,
            new: G::Shrink,
            cause: Cause<<P::Output as Prove>::Error>,
            pin: Pin<Box<P>>,
        },
        Done,
    }

    impl<G: Generate<Shrink: Unpin> + Unpin> Checker<G, Run> {
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
                shrinks: 0..self.shrink.count,
                machine: Machine::Generate {
                    generator: self.generator,
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
                        pin,
                    } => {
                        let Some(mut state) = states.next() else {
                            break Poll::Ready(None);
                        };
                        let shrinker = generator.generate(&mut state);
                        let pin = prepare(shrinker.item(), &mut checks.check, pin);
                        checks.machine = Machine::Handle1 {
                            generator,
                            states,
                            state,
                            shrinker,
                            pin,
                        };
                    }
                    Machine::Handle1 {
                        generator,
                        states,
                        state,
                        shrinker,
                        mut pin,
                    } => match ready!(handle(pin.as_mut(), cx)) {
                        Ok(proof) => {
                            checks.machine = Machine::Generate {
                                generator,
                                states,
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
                                shrinker,
                                cause,
                                pin: Some(pin),
                            };
                        }
                    },
                    Machine::Shrink {
                        index,
                        state,
                        mut shrinker,
                        cause,
                        pin,
                    } => {
                        let next = match checks.shrinks.next() {
                            Some(index) => index,
                            None => {
                                checks.machine = Machine::Done;
                                break Poll::Ready(Some(fail(
                                    shrinker.item(),
                                    index,
                                    state,
                                    cause,
                                )));
                            }
                        };
                        let new = match shrinker.shrink() {
                            Some(shrinker) => shrinker,
                            None => {
                                checks.machine = Machine::Done;
                                break Poll::Ready(Some(fail(
                                    shrinker.item(),
                                    index,
                                    state,
                                    cause,
                                )));
                            }
                        };
                        let pin = prepare(new.item(), &mut checks.check, pin);
                        checks.machine = Machine::Handle2 {
                            index: next,
                            state,
                            old: shrinker,
                            new,
                            cause,
                            pin,
                        };
                    }
                    Machine::Handle2 {
                        index,
                        state,
                        old,
                        new,
                        cause,
                        mut pin,
                    } => match ready!(handle(pin.as_mut(), cx)) {
                        Ok(proof) => {
                            checks.machine = Machine::Shrink {
                                index,
                                state: state.clone(),
                                shrinker: old,
                                cause,
                                pin: Some(pin),
                            };
                            if checks.yields.1 {
                                break Poll::Ready(Some(shrink(new.item(), index, state, proof)));
                            }
                        }
                        Err(new_cause) => {
                            checks.machine = Machine::Shrink {
                                index,
                                state: state.clone(),
                                shrinker: new,
                                cause: new_cause,
                                pin: Some(pin),
                            };
                            if checks.yields.2 {
                                break Poll::Ready(Some(shrunk(old.item(), index, state, cause)));
                            }
                        }
                    },
                    Machine::Done => break Poll::Ready(None),
                }
            }
        }
    }

    fn prepare<T, P, F: FnMut(T) -> P>(
        item: T,
        mut check: F,
        pin: Option<Pin<Box<P>>>,
    ) -> Pin<Box<P>> {
        match pin {
            Some(mut pin) => {
                pin.set(check(item));
                pin
            }
            None => Box::pin(check(item)),
        }
    }

    fn handle<P: Prove, F: Future<Output = P>>(
        check: Pin<&mut F>,
        context: &mut Context,
    ) -> Poll<result::Result<P::Proof, Cause<P::Error>>> {
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
