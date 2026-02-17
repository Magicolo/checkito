use crate::{
    generate::Generate,
    prove::Prove,
    shrink::Shrink,
    state::{self, Modes, Sizes, State, States},
    utility::cast,
    GENERATES, SHRINKS,
};
use core::{
    fmt::{self, Debug},
    ops::{self, Deref, DerefMut},
    panic::AssertUnwindSafe,
    result,
};
use std::{borrow::Cow, error, panic::catch_unwind};
use std::{iter, mem::take};
use std::{num::NonZeroUsize, thread::available_parallelism};

/// Configures the generation process.
#[derive(Clone, Debug)]
pub struct Generates {
    /// The seed for the random number generator.
    ///
    /// Using the same seed will cause the generator to produce the same
    /// sequence of random values, making test runs reproducible. It
    /// defaults to a random value.
    pub seed: u64,
    /// The range of sizes (`0.0..=1.0`) that will be gradually traversed while
    /// generating values.
    ///
    /// Defaults to `0.0..=1.0`.
    pub sizes: Sizes,
    /// The maximum number of values to generate and test.
    ///
    /// Setting this to `0` will prevent any tests from running. Defaults to
    /// [`GENERATES`].
    pub count: usize,
    /// Whether the iterator should yield [`Result::Pass`] items.
    ///
    /// If `false`, the iterator will be empty for successful test runs.
    /// Defaults to `true`.
    pub items: bool,
    /// Overrides the exhaustive check detection.
    /// - `Some(true)`: Forces exhaustive checking, ignoring `seed` and `sizes`.
    /// - `Some(false)`: Forces random sampling.
    /// - `None`: Automatically determines whether to be exhaustive based on
    ///   whether the generator's [`Generate::cardinality`] is less than or
    ///   equal to `count`.
    pub exhaustive: Option<bool>,
}

/// Configures the shrinking process.
#[derive(Clone, Debug)]
pub struct Shrinks {
    /// The maximum number of times to shrink a failing value.
    ///
    /// Setting this to `0` disables shrinking. Defaults to [`SHRINKS`].
    pub count: usize,
    /// Whether the iterator should yield [`Result::Shrink`] items.
    ///
    /// If `false`, successful shrink steps will not be reported. Defaults to
    /// `true`.
    pub items: bool,
    /// Whether the iterator should yield [`Result::Shrunk`] items.
    ///
    /// If `false`, failing shrink steps will not be reported. Defaults to
    /// `true`.
    pub errors: bool,
}

/// Holds a generator and the configuration for the checking and shrinking
/// processes.
///
/// A `Checker` is created by calling [`Check::checker`] on a generator. It
/// provides a builder-like interface for configuring the test run before
/// executing it via [`Checker::check`] or [`Checker::checks`].
#[derive(Debug, Clone)]
pub struct Checker<G: ?Sized, R> {
    /// The configuration for the generation process.
    pub generate: Generates,
    /// The configuration for the shrinking process.
    pub shrink: Shrinks,
    /// The configuration for the running process.
    run: R,
    /// The generator that will produce values for the test.
    pub generator: G,
}

#[derive(Debug, Copy, Clone)]
struct Yields {
    passes: bool,
    shrinks: bool,
    shrunks: bool,
}

/// An extension trait, implemented for all [`Generate`] types, that provides
/// the main entry points for running property tests.
pub trait Check: Generate {
    /// Creates a [`Checker`] for this generator.
    ///
    /// The `Checker` can be used to configure and run the property test.
    ///
    /// # Examples
    ///
    /// ```
    /// # use checkito::*;
    /// let mut checker = (0..100).checker();
    /// checker.generate.count = 500; // Run 500 test cases.
    /// checker.shrink.count = 0; // Disable shrinking.
    ///
    /// let result = checker.check(|x| x < 100);
    /// assert!(result.is_none());
    /// ```
    fn checker(self) -> Checker<Self, synchronous::Run>
    where
        Self: Sized,
    {
        Checker::new(self, state::seed())
    }

    /// Creates an iterator that runs the property test.
    ///
    /// This is useful for consuming the full sequence of test results,
    /// including intermediate shrink steps.
    ///
    /// # Examples
    ///
    /// ```
    /// # use checkito::*;
    /// let mut checks = (0..10).checks(|x| x < 5);
    ///
    /// assert!(matches!(checks.next(), Some(check::Result::Pass(_))));
    /// // ...
    /// assert!(matches!(checks.last(), Some(check::Result::Fail(_))));
    /// ```
    fn checks<P: Prove, C: FnMut(Self::Item) -> P>(
        self,
        check: C,
    ) -> synchronous::Iterator<Self, P, C>
    where
        Self: Sized,
    {
        self.checker().checks(check)
    }

    /// Runs the property test and returns the final failure, if any.
    ///
    /// This is the simplest way to run a test. It consumes the entire test
    /// iterator and returns `Some(Fail)` if the property was violated, or
    /// `None` if all test cases passed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use checkito::*;
    /// let success = (0..100).check(|x| x < 100);
    /// assert!(success.is_none());
    ///
    /// let failure = (0..100).check(|x| x < 50);
    /// assert!(failure.is_some());
    ///
    /// let fail = failure.unwrap();
    /// // The shrinker will find the minimal failing value.
    /// assert_eq!(fail.item, 50);
    /// ```
    fn check<P: Prove, C: FnMut(Self::Item) -> P>(
        &self,
        check: C,
    ) -> Option<Fail<Self::Item, P::Error>> {
        self.checker().check(check)
    }
}

/// The result of a single step in the property testing process.
#[derive(Clone, Debug)]
pub enum Result<T, P: Prove> {
    /// A generated value passed the test.
    Pass(Pass<T, P::Proof>),
    /// A shrunk value passed the test, meaning it did not reproduce the
    /// failure.
    Shrink(Pass<T, P::Proof>),
    /// A shrunk value failed the test, becoming the new minimal failing case.
    Shrunk(Fail<T, P::Error>),
    /// The final, minimal value that failed the test after shrinking is
    /// complete.
    Fail(Fail<T, P::Error>),
}

/// Represents a successful test case.
#[derive(Clone, Debug)]
pub struct Pass<T, P> {
    /// The value that passed the test.
    pub item: T,
    /// The proof produced by the [`Prove`] implementation (e.g., `()` or the
    /// `Ok` variant of a `Result`).
    pub proof: P,
    /// The number of generations that occurred before this pass.
    pub generates: usize,
    /// The number of shrinks that occurred before this pass.
    pub shrinks: usize,
    /// The generator state that produced the item.
    pub state: State,
}

/// Represents a failed test case.
#[derive(Clone, Debug)]
pub struct Fail<T, E> {
    /// The value that failed the test.
    pub item: T,
    /// The reason for the failure.
    pub cause: Cause<E>,
    /// The number of generations that occurred before this failure.
    pub generates: usize,
    /// The number of shrinks that occurred before this failure.
    pub shrinks: usize,
    /// The generator state that produced the failing item.
    pub state: State,
}

/// The cause of a check failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cause<E> {
    /// The property was disproven by the test function's return value
    /// (e.g., it returned `false` or `Err`).
    Disprove(E),
    /// The test function panicked.
    Panic(Option<Cow<'static, str>>),
}

impl<G: Generate + ?Sized> Check for G {}

impl<G: Generate> Checker<G, synchronous::Run> {
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
            run: synchronous::Run,
        }
    }
}

impl<G: Generate, R> Checker<G, R> {
    fn with<S>(self, run: S) -> Checker<G, S> {
        Checker {
            generate: self.generate,
            shrink: self.shrink,
            generator: self.generator,
            run,
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

    pub fn pass(&self, shrink: bool) -> Option<&Pass<T, P::Proof>> {
        match self {
            Result::Pass(pass) => Some(pass),
            Result::Shrink(pass) if shrink => Some(pass),
            _ => None,
        }
    }

    pub fn into_pass(self, shrink: bool) -> result::Result<Pass<T, P::Proof>, Self> {
        match self {
            Result::Pass(pass) => Ok(pass),
            Result::Shrink(pass) if shrink => Ok(pass),
            result => Err(result),
        }
    }

    pub fn fail(&self, shrunk: bool) -> Option<&Fail<T, P::Error>> {
        match self {
            Result::Fail(fail) => Some(fail),
            Result::Shrunk(fail) if shrunk => Some(fail),
            _ => None,
        }
    }

    pub fn into_fail(self, shrunk: bool) -> result::Result<Fail<T, P::Error>, Self> {
        match self {
            Result::Fail(fail) => Ok(fail),
            Result::Shrunk(fail) if shrunk => Ok(fail),
            result => Err(result),
        }
    }

    pub fn into_item(self) -> T {
        match self {
            Result::Pass(pass) | Result::Shrink(pass) => pass.item,
            Result::Fail(fail) | Result::Shrunk(fail) => fail.item,
        }
    }

    #[allow(clippy::result_large_err)]
    pub fn into_result(self) -> result::Result<Pass<T, P::Proof>, Fail<T, P::Error>> {
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
        P: Debug,
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

impl<T: Debug, E: Debug> fmt::Display for Fail<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl<T: Debug, E: Debug> error::Error for Fail<T, E> {}

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

fn catch<T, E, F: FnOnce() -> T>(run: F) -> result::Result<T, Cause<E>> {
    catch_unwind(AssertUnwindSafe(move || run())).map_err(|error| Cause::Panic(cast(error).ok()))
}

pub(crate) mod synchronous {
    use super::*;

    pub struct Run;

    /// An iterator over the results of a property test.
    ///
    /// The iterator first enters a generation phase, where it produces values and
    /// runs the test against them.
    ///
    /// - If a check passes, it yields a [`Result::Pass`].
    /// - If a check fails, it enters the shrinking phase.
    ///
    /// In the shrinking phase, it repeatedly tries to simplify the failing value.
    ///
    /// - If a shrunk value passes the test, it yields a [`Result::Shrink`],
    ///   indicating that this simpler value did not reproduce the failure.
    /// - If a shrunk value fails the test, it yields a [`Result::Shrunk`], and this
    ///   new, simpler value becomes the one to be shrunk further.
    ///
    /// Once a value can no longer be shrunk, the iterator yields a final
    /// [`Result::Fail`] and then terminates.
    pub struct Iterator<G: Generate, P: Prove, C> {
        yields: Yields,
        check: C,
        machine: Machine<G, P>,
    }

    enum Machine<G: Generate, P: Prove> {
        Generate {
            generator: G,
            states: States,
            shrinks: usize,
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

    impl<G: Generate, P: Prove> Default for Machine<G, P> {
        fn default() -> Self {
            Self::Done
        }
    }

    impl<G: Generate> Checker<G, Run> {
        #[cfg(feature = "asynchronous")]
        pub fn asynchronous(self, concurrency: Option<usize>) -> Checker<G, asynchronous::Run> {
            self.with(asynchronous::Run {
                concurrency: concurrency
                    .unwrap_or(available_parallelism().map_or(8, NonZeroUsize::get)),
            })
        }

        pub fn check<P: Prove, C: FnMut(G::Item) -> P>(
            mut self,
            check: C,
        ) -> Option<Fail<G::Item, P::Error>> {
            self.generate.items = false;
            self.shrink.items = false;
            self.shrink.errors = false;
            self.checks(check).last()?.into_fail(false).ok()
        }

        pub fn checks<P: Prove, C: FnMut(G::Item) -> P>(self, check: C) -> Iterator<G, P, C> {
            let modes = Modes::with(
                self.generate.count,
                self.generate.sizes,
                self.generate.seed,
                self.generator.cardinality(),
                self.generate.exhaustive,
            );
            Iterator {
                check,
                yields: Yields {
                    passes: self.generate.items,
                    shrinks: self.shrink.items,
                    shrunks: self.shrink.errors,
                },
                machine: Machine::Generate {
                    generator: self.generator,
                    states: modes.into(),
                    shrinks: self.shrink.count,
                },
            }
        }
    }

    impl<G: Generate, P: Prove, C: FnMut(G::Item) -> P> iter::Iterator for Iterator<G, P, C> {
        type Item = Result<G::Item, P>;

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                match take(&mut self.machine) {
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
                                if self.yields.passes {
                                    break Some(pass(shrinker.item(), state, proof));
                                }
                            }
                            Err(cause) => {
                                self.machine = Machine::Shrink {
                                    index: 0,
                                    state,
                                    shrinker,
                                    shrinks: 0..shrinks,
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
                            None => break Some(fail(old_shrinker.item(), index, state, old_cause)),
                        };
                        let new_shrinker = match old_shrinker.shrink() {
                            Some(shrinker) => shrinker,
                            None => break Some(fail(old_shrinker.item(), index, state, old_cause)),
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
                                if self.yields.shrinks {
                                    break Some(shrink(new_shrinker.item(), next, state, proof));
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
                                if self.yields.shrunks {
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

    fn handle<T, P: Prove, C: FnMut(T) -> P>(
        item: T,
        mut check: C,
    ) -> result::Result<P::Proof, Cause<P::Error>> {
        catch(move || check(item))?.prove().map_err(Cause::Disprove)
    }
}

#[cfg(feature = "asynchronous")]
pub(crate) mod asynchronous {
    use super::*;
    use core::{
        future::Future,
        pin::Pin,
        task::{ready, Context, Poll},
    };
    use futures_lite::{stream, StreamExt};
    use pin_project_lite::pin_project;
    use std::collections::VecDeque;

    pub struct Run {
        pub(crate) concurrency: usize,
    }

    pin_project! {
        /// An iterator over the results of a property test.
        ///
        /// The iterator first enters a generation phase, where it produces values and
        /// runs the test against them.
        ///
        /// - If a check passes, it yields a [`Result::Pass`].
        /// - If a check fails, it enters the shrinking phase.
        ///
        /// In the shrinking phase, it repeatedly tries to simplify the failing value.
        ///
        /// - If a shrunk value passes the test, it yields a [`Result::Shrink`],
        ///   indicating that this simpler value did not reproduce the failure.
        /// - If a shrunk value fails the test, it yields a [`Result::Shrunk`], and this
        ///   new, simpler value becomes the one to be shrunk further.
        ///
        /// Once a value can no longer be shrunk, the iterator yields a final
        /// [`Result::Fail`] and then terminates.
        pub struct Stream<G: Generate, P: Future<Output: Prove>, C> {
            yields: Yields,
            check: C,
            #[pin]
            machine: Machine<G, P>,
        }
    }

    pin_project! {
        struct Proving<S, P> {
            state: State,
            shrinker: S,
            #[pin]
            prove: P,
        }
    }

    pin_project! {
        #[project = MachineProjection]
        enum Machine<G: Generate, P: Future<Output: Prove>> {
            Generate {
                generator: G,
                next: usize,
                states: States,
                shrinks: usize,
                #[pin]
                proving: Box<[Option<Proving<G::Shrink, P>>]>,
            },
            Shrink {
                index: usize,
                state: State,
                concurrency: usize,
                shrinks: ops::Range<usize>,
                shrinker: G::Shrink,
                cause: Cause<<P::Output as Prove>::Error>,
                #[pin]
                proving: VecDeque<Proving<G::Shrink, P>>,
            },
            Done,
        }
    }

    impl<G: Generate> Checker<G, Run> {
        pub fn synchronous(self) -> Checker<G, synchronous::Run> {
            self.with(synchronous::Run)
        }

        pub async fn check<P: Future<Output: Prove>, C: FnMut(G::Item) -> P>(
            mut self,
            check: C,
        ) -> Option<Fail<G::Item, <P::Output as Prove>::Error>> {
            self.generate.items = false;
            self.shrink.items = false;
            self.shrink.errors = false;
            self.checks(check).last().await?.into_fail(false).ok()
        }

        pub fn checks<P: Future<Output: Prove>, C: FnMut(G::Item) -> P>(
            self,
            check: C,
        ) -> Stream<G, P, C> {
            let modes = Modes::with(
                self.generate.count,
                self.generate.sizes,
                self.generate.seed,
                self.generator.cardinality(),
                self.generate.exhaustive,
            );
            Stream {
                yields: Yields {
                    passes: self.generate.items,
                    shrinks: self.shrink.items,
                    shrunks: self.shrink.errors,
                },
                machine: Machine::Generate {
                    generator: self.generator,
                    shrinks: self.shrink.count,
                    states: modes.into(),
                    next: 0,
                    proving: vec![None; self.run.concurrency].into_boxed_slice(),
                },
                check,
            }
        }
    }

    impl<G: Generate, P: Future<Output: Prove>, C: FnMut(G::Item) -> P> stream::Stream
        for Stream<G, P, C>
    {
        type Item = Result<G::Item, P::Output>;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let mut this = self.project();
            loop {
                match this.machine.as_mut().project() {
                    MachineProjection::Generate {
                        generator,
                        states,
                        concurrency,
                        shrinks,
                        proving,
                    } => {
                        while proving.len() < *concurrency {
                            if let Some(mut state) = states.next() {
                                let shrinker = generator.generate(&mut state);
                                let prove = (this.check)(shrinker.item());
                                proving.push(Proving {
                                    state,
                                    shrinker,
                                    prove: Box::pin(prove),
                                });
                            }
                        }
                        if proving.is_empty() {
                            break Poll::Ready(None);
                        }
                        let shrinker = generator.generate(&mut state);
                        this.machine.set(Machine::Done);
                        todo!();
                    }
                    MachineProjection::Shrink {
                        index,
                        state,
                        concurrency,
                        shrinks,
                        shrinker,
                        cause,
                        proving,
                    } => todo!(),
                    MachineProjection::Done => todo!(),
                }
                // match replace(&mut checks.machine, Machine::Done) {
                //     Machine::Generate {
                //         generator,
                //         mut states,
                //         shrinks,
                //         mut pin,
                //     } => {
                //         let Some(mut state) = states.next() else {
                //             break Poll::Ready(None);
                //         };
                //         let shrinker = generator.generate(&mut state);
                //         match prepare(shrinker.item(), &mut checks.check, &mut pin) {
                //             Ok(pin) => {
                //                 checks.machine = Machine::Handle1 {
                //                     generator,
                //                     states,
                //                     state,
                //                     shrinks,
                //                     shrinker,
                //                     pin,
                //                 }
                //             }
                //             Err(cause) => {
                //                 checks.machine = Machine::Shrink {
                //                     index: 0,
                //                     state,
                //                     shrinks: 0..shrinks,
                //                     shrinker,
                //                     cause,
                //                     pin,
                //                 }
                //             }
                //         };
                //     }
                //     Machine::Handle1 {
                //         generator,
                //         states,
                //         state,
                //         shrinks,
                //         shrinker,
                //         mut pin,
                //     } => match ready!(handle(pin.as_mut(), cx)) {
                //         Ok(proof) => {
                //             checks.machine = Machine::Generate {
                //                 generator,
                //                 states,
                //                 shrinks,
                //                 pin: Some(pin),
                //             };
                //             if checks.yields.0 {
                //                 break Poll::Ready(Some(pass(shrinker.item(), state, proof)));
                //             }
                //         }
                //         Err(cause) => {
                //             checks.machine = Machine::Shrink {
                //                 index: 0,
                //                 state,
                //                 shrinks: 0..shrinks,
                //                 shrinker,
                //                 cause,
                //                 pin: Some(pin),
                //             };
                //         }
                //     },
                //     Machine::Shrink {
                //         index,
                //         state,
                //         mut shrinks,
                //         shrinker: mut old_shrinker,
                //         cause: old_cause,
                //         mut pin,
                //     } => {
                //         let next = match shrinks.next() {
                //             Some(index) => index,
                //             None => {
                //                 checks.machine = Machine::Done;
                //                 break Poll::Ready(Some(fail(
                //                     old_shrinker.item(),
                //                     index,
                //                     state,
                //                     old_cause,
                //                 )));
                //             }
                //         };
                //         let new_shrinker = match old_shrinker.shrink() {
                //             Some(shrinker) => shrinker,
                //             None => {
                //                 checks.machine = Machine::Done;
                //                 break Poll::Ready(Some(fail(
                //                     old_shrinker.item(),
                //                     index,
                //                     state,
                //                     old_cause,
                //                 )));
                //             }
                //         };
                //         match prepare(new_shrinker.item(), &mut checks.check, &mut pin) {
                //             Ok(pin) => {
                //                 checks.machine = Machine::Handle2 {
                //                     index: next,
                //                     state,
                //                     old: old_shrinker,
                //                     new: new_shrinker,
                //                     shrinks,
                //                     cause: old_cause,
                //                     pin,
                //                 }
                //             }
                //             Err(new_cause) => {
                //                 checks.machine = Machine::Shrink {
                //                     index: next,
                //                     state: state.clone(),
                //                     shrinks,
                //                     shrinker: new_shrinker,
                //                     cause: new_cause,
                //                     pin,
                //                 };
                //                 if checks.yields.2 {
                //                     break Poll::Ready(Some(shrunk(
                //                         old_shrinker.item(),
                //                         next,
                //                         state,
                //                         old_cause,
                //                     )));
                //                 }
                //             }
                //         }
                //     }
                //     Machine::Handle2 {
                //         index,
                //         state,
                //         old,
                //         new,
                //         shrinks,
                //         cause,
                //         mut pin,
                //     } => match ready!(handle(pin.as_mut(), cx)) {
                //         Ok(proof) => {
                //             checks.machine = Machine::Shrink {
                //                 index,
                //                 state: state.clone(),
                //                 shrinks,
                //                 shrinker: old,
                //                 cause,
                //                 pin: Some(pin),
                //             };
                //             if checks.yields.1 {
                //                 break Poll::Ready(Some(shrink(new.item(), index, state, proof)));
                //             }
                //         }
                //         Err(new_cause) => {
                //             checks.machine = Machine::Shrink {
                //                 index,
                //                 state: state.clone(),
                //                 shrinks,
                //                 shrinker: new,
                //                 cause: new_cause,
                //                 pin: Some(pin),
                //             };
                //             if checks.yields.2 {
                //                 break Poll::Ready(Some(shrunk(old.item(), index, state, cause)));
                //             }
                //         }
                //     },
                //     Machine::Done => break Poll::Ready(None),
                // }
            }
        }
    }

    fn prepare<T, P: Future<Output: Prove>, C: FnMut(T) -> P>(
        item: T,
        mut check: C,
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
            Err(error) => Err(Cause::Panic(cast(error).ok())),
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
            Err(error) => Poll::Ready(Err(Cause::Panic(cast(error).ok()))),
        }
    }
}
