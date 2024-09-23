use crate::{generate::State, prove::Prove, random::Random, shrink::Shrink, Generate};
use std::{
    borrow::Cow,
    error, fmt,
    num::NonZeroUsize,
    panic::{self, AssertUnwindSafe},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    thread::{available_parallelism, scope},
    time::{Duration, Instant},
};

/// Bounds the shrinking process.
#[derive(Clone, Copy, Debug)]
pub struct Shrinks {
    /// Maximum number of successful attempts at reducing the 'size' of the input before aborting the shrinking process.
    pub accept: usize,
    /// Maximum number of failed attempts at reducing the 'size' of the input before aborting the shrinking process.
    pub reject: usize,
    /// Maximum time spent shrinking.
    pub duration: Duration,
}

/// The [`Checker`] structure holds a reference to a [`Generate`] instance and some configuration options for the checking and shrinking processes.
#[derive(Debug)]
pub struct Checker<'a, G: ?Sized> {
    /// A generator that will provide the values and shrinkers for the checking and shrinking processes.
    pub generator: &'a G,
    /// Maximum number of errors that the results of a `check` call will contain. When it is reached, the checking process aborts.
    /// Defaults to 1.
    pub errors: usize,
    /// Limits the shrinking process.
    /// Defaults to a duration limit of 30 seconds.
    pub shrinks: Shrinks,
    /// Seed for the random number generator used to generate random primitives.
    pub seed: Option<u64>,
}

/// A structure representing a series of checks to be performed on a generator.
///
/// This structure is used to iterate over a sequence of checks, where each check
/// is performed on a generated item. It keeps track of the number of errors
/// encountered and the number of checks remaining.
#[derive(Debug)]
pub struct Checks<'a, G: ?Sized, F> {
    checker: Checker<'a, G>,
    random: Random,
    errors: usize,
    index: usize,
    count: usize,
    check: F,
}

#[derive(Clone, Debug)]
/// An error produced by a check failure.
/// A check fails when a proof `P` is `false` for a given generated value.
pub struct Error<T, P> {
    /// The generator state that caused the error.
    pub state: State,
    pub item: T,
    pub cause: Cause<P>,
    pub shrinks: Shrinks,
}

/// The cause of a check failure.
/// A check fails when a proof `P` is `false` for a given generated value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cause<P> {
    /// A `Disprove` cause is a value that, when checked, returns a value of type `P`
    /// that does not satisfy the property.
    Disprove(P),
    /// A `Panic` cause is produced when a check panics during its evaluation.
    /// The message associated with the panic is included if it can be casted to a string.
    Panic(Option<Cow<'static, str>>),
}

impl<'a, G: ?Sized> Checker<'a, G> {
    pub const fn new(generator: &'a G) -> Self {
        Self {
            generator,
            errors: 1,
            shrinks: Shrinks {
                accept: usize::MAX,
                reject: usize::MAX,
                duration: Duration::from_secs(30),
            },
            seed: None,
        }
    }
}

impl<G: ?Sized> Clone for Checker<'_, G> {
    fn clone(&self) -> Self {
        Self {
            generator: self.generator,
            errors: self.errors,
            shrinks: self.shrinks,
            seed: self.seed,
        }
    }
}

impl<G: ?Sized, F: Clone> Clone for Checks<'_, G, F> {
    fn clone(&self) -> Self {
        Self {
            checker: self.checker.clone(),
            random: self.random.clone(),
            check: self.check.clone(),
            errors: self.errors,
            index: self.index,
            count: self.count,
        }
    }
}

impl<'a, G: Generate + ?Sized> Checker<'a, G> {
    pub fn check<P: Prove, F: FnMut(&G::Item) -> P>(
        &self,
        size: f64,
        check: F,
    ) -> Result<G::Item, Error<G::Item, P>> {
        next(
            self.generator,
            State::new(size, self.seed),
            self.shrinks,
            check,
        )
    }

    pub fn checks<P: Prove, F: FnMut(&G::Item) -> P>(
        &self,
        count: usize,
        check: F,
    ) -> Checks<'a, G, F> {
        Checks {
            checker: self.clone(),
            random: Random::new(self.seed),
            errors: 0,
            index: 0,
            count,
            check,
        }
    }
}

impl<'a, G: Generate + ?Sized + Sync> Checker<'a, G>
where
    G::Item: Send,
{
    pub fn check_parallel<'b, P: Prove + Send + 'b, F: Fn(&G::Item) -> P + Sync + 'b>(
        &'b self,
        count: usize,
        parallel: Option<usize>,
        check: F,
    ) -> impl Iterator<Item = Result<G::Item, Error<G::Item, P>>> + 'b {
        let parallel = match parallel {
            Some(parallel) => parallel.max(1),
            None => available_parallelism().map_or(1, NonZeroUsize::get),
        };
        let results = Mutex::new(Vec::with_capacity(count));
        let errors = AtomicUsize::new(0);
        let mut random = Random::new(self.seed);
        let capacity = divide_ceiling(count, parallel);
        if capacity <= 8 || count < 32 {
            batch(
                self.generator,
                &results,
                (0, 1, count),
                self.shrinks,
                (&errors, self.errors),
                &mut random,
                check,
            );
        } else {
            scope(|scope| {
                for offset in 1..parallel {
                    let check = &check;
                    let errors = &errors;
                    let results = &results;
                    let seed = random.u64(..);
                    scope.spawn(move || {
                        batch(
                            self.generator,
                            results,
                            (offset, parallel, count),
                            self.shrinks,
                            (errors, self.errors),
                            &mut Random::new(Some(seed)),
                            check,
                        )
                    });
                }

                batch(
                    self.generator,
                    &results,
                    (0, parallel, count),
                    self.shrinks,
                    (&errors, self.errors),
                    &mut random,
                    &check,
                )
            });
        }
        results.into_inner().unwrap().into_iter()
    }
}

impl<G: Generate + ?Sized, P: Prove, F: FnMut(&G::Item) -> P> Iterator for Checks<'_, G, F> {
    type Item = Result<G::Item, Error<G::Item, P>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count || self.errors >= self.checker.errors {
            None
        } else {
            let result = next(
                self.checker.generator,
                State::from_iteration(self.index, self.count, Some(self.random.u64(..))),
                self.checker.shrinks,
                &mut self.check,
            );
            self.index += 1;
            match result {
                Ok(item) if self.errors < self.checker.errors => Some(Ok(item)),
                Err(error) if self.errors < self.checker.errors => {
                    self.errors += 1;
                    Some(Err(error))
                }
                _ => None,
            }
        }
    }
}

fn handle<T, P: Prove, F: FnMut(&T) -> P>(item: &T, mut check: F) -> Option<Cause<P>> {
    let error = match panic::catch_unwind(AssertUnwindSafe(|| check(item))) {
        Ok(prove) if prove.prove() => return None,
        Ok(prove) => return Some(Cause::Disprove(prove)),
        Err(error) => error,
    };
    let error = match error.downcast::<&'static str>() {
        Ok(error) => return Some(Cause::Panic(Some(Cow::Borrowed(*error)))),
        Err(error) => error,
    };
    let error = match error.downcast::<String>() {
        Ok(error) => return Some(Cause::Panic(Some(Cow::Owned(*error)))),
        Err(error) => error,
    };
    let error = match error.downcast::<Box<str>>() {
        Ok(error) => return Some(Cause::Panic(Some(Cow::Owned(error.to_string())))),
        Err(error) => error,
    };
    match error.downcast::<Cow<'static, str>>() {
        Ok(error) => Some(Cause::Panic(Some(*error))),
        Err(_) => Some(Cause::Panic(None)),
    }
}

fn shrink<S: Shrink, P: Prove, F: FnMut(&S::Item) -> P>(
    shrinks: Shrinks,
    shrinker: &mut S,
    error: &mut Error<S::Item, P>,
    check: F,
) -> Option<bool> {
    if error.shrinks.reject < shrinks.reject
        && error.shrinks.accept < shrinks.accept
        && error.shrinks.duration < shrinks.duration
    {
        let now = Instant::now();
        let shrunk = shrinker.shrink()?;
        let item = shrunk.item();
        match handle(&item, check) {
            Some(cause) => {
                *shrinker = shrunk;
                error.item = item;
                error.cause = cause;
                error.shrinks.reject = 0;
                error.shrinks.accept += 1;
                error.shrinks.duration += Instant::now() - now;
                Some(true)
            }
            _ => {
                error.shrinks.reject += 1;
                error.shrinks.duration += Instant::now() - now;
                Some(false)
            }
        }
    } else {
        None
    }
}

fn next<G: Generate + ?Sized, P: Prove, F: FnMut(&G::Item) -> P>(
    generator: &G,
    mut state: State,
    shrinks: Shrinks,
    mut check: F,
) -> Result<G::Item, Error<G::Item, P>> {
    let mut shrinker = generator.generate(&mut state);
    let item = shrinker.item();
    let Some(cause) = handle(&item, &mut check) else {
        return Ok(item);
    };
    let mut error = Error {
        state,
        cause,
        item,
        shrinks: Shrinks {
            accept: 0,
            reject: 0,
            duration: Duration::ZERO,
        },
    };
    while shrink(shrinks, &mut shrinker, &mut error, &mut check).is_some() {}
    Err(error)
}

type Results<G, P> = Mutex<Vec<Result<<G as Generate>::Item, Error<<G as Generate>::Item, P>>>>;

fn batch<G: Generate + ?Sized, P: Prove, F: Fn(&G::Item) -> P>(
    generator: &G,
    results: &Results<G, P>,
    (offset, step, count): (usize, usize, usize),
    shrinks: Shrinks,
    errors: (&AtomicUsize, usize),
    random: &mut Random,
    check: F,
) {
    for index in (offset..count).step_by(step) {
        let state = State::from_iteration(index, count, Some(random.u64(..)));
        match next(generator, state, shrinks, &check) {
            Ok(item) if errors.0.load(Ordering::Relaxed) < errors.1 => {
                results.lock().unwrap().push(Ok(item))
            }
            Err(error) if errors.0.fetch_add(1, Ordering::Relaxed) < errors.1 => {
                results.lock().unwrap().push(Err(error))
            }
            _ => break,
        }
    }
}

const fn divide_ceiling(left: usize, right: usize) -> usize {
    let value = left / right;
    let remain = left % right;
    if remain > 0 && right > 0 {
        value + 1
    } else {
        value
    }
}

impl<T: fmt::Debug, P: fmt::Debug> fmt::Display for Error<T, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<T: fmt::Debug, P: fmt::Debug> error::Error for Error<T, P> {}
