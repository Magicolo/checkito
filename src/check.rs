use crate::{generate::State, prove::Prove, shrink::Shrink, Generate};
use fastrand::Rng;
use std::{
    error, fmt,
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering},
    thread::{available_parallelism, scope},
};

#[derive(Clone, Copy, Debug)]
pub struct Shrinks {
    /// Maximum number of successful attemps at reducing the 'size' of the input before aborting the shrinking process.
    pub accept: usize,
    /// Maximum number of failed attempts at reducing the 'size' of the input before aborting the shrinking process.
    pub reject: usize,
}

pub struct Checker<'a, G: ?Sized> {
    /// A generator that will provide the values and shrinkers for the checking and shrinking processes.
    pub generator: &'a G,
    /// Number of iterations that the checking process will run for.
    pub checks: usize,
    /// Maximum number of errors that the results of a `check` call will contain. When it is reached, the checking process aborts.
    pub errors: usize,
    /// Limits the shrinking process.
    pub shrinks: Shrinks,
    /// Seed for the random number generator used to generate random primitives.
    pub seed: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct Error<T, P> {
    pub index: usize,
    pub count: usize,
    /// The generator state that generated the error.
    pub state: State,
    pub original: (T, P),
    pub shrinks: Shrinks,
    pub shrunk: Option<(T, P)>,
}

impl<T, P> Error<T, P> {
    pub fn original(&self) -> &T {
        &self.original.0
    }

    pub fn shrunk(&self) -> &T {
        &self.shrunk.as_ref().unwrap_or(&self.original).0
    }
}

impl<'a, G: ?Sized> Checker<'a, G> {
    pub const fn new(generator: &'a G, count: usize) -> Self {
        Self {
            generator,
            checks: count,
            errors: 1,
            shrinks: Shrinks {
                accept: usize::MAX,
                reject: usize::MAX,
            },
            seed: None,
        }
    }
}

impl<G: ?Sized> Clone for Checker<'_, G> {
    fn clone(&self) -> Self {
        Self {
            generator: self.generator,
            checks: self.checks,
            errors: self.errors,
            shrinks: self.shrinks,
            seed: self.seed,
        }
    }
}

impl<'a, G: Generate + ?Sized> Checker<'a, G> {
    pub fn check_sequential<'b, P: Prove + 'b, F: FnMut(&G::Item) -> P + 'b>(
        &'b self,
        mut check: F,
    ) -> impl Iterator<Item = Result<G::Item, Error<G::Item, P>>> + 'b {
        let random = self.seed.map_or_else(Rng::new, Rng::with_seed);
        let mut errors = (0, self.errors);
        Iterator::map_while(0..self.checks, move |index| {
            match next(
                self.generator,
                index,
                self.checks,
                self.shrinks,
                &random,
                &mut check,
            ) {
                Ok(item) if errors.0 < errors.1 => Some(Ok(item)),
                Err(error) if errors.0 < errors.1 => {
                    errors.0 += 1;
                    Some(Err(error))
                }
                _ => None,
            }
        })
    }
}

impl<'a, G: Generate + ?Sized + Sync> Checker<'a, G>
where
    G::Item: Send,
{
    pub fn check_parallel<'b, P: Prove + Send + 'b, F: Fn(&G::Item) -> P + Sync + 'b>(
        &'b self,
        parallel: Option<usize>,
        check: F,
    ) -> impl Iterator<Item = Result<G::Item, Error<G::Item, P>>> + 'b {
        let parallel = match parallel {
            Some(parallel) => parallel.max(1),
            None => available_parallelism().map_or(1, NonZeroUsize::get),
        };
        let mut results = Vec::with_capacity(self.checks);
        let errors = AtomicUsize::new(0);
        let random = self.seed.map_or_else(Rng::new, Rng::with_seed);
        let capacity = divide_ceiling(self.checks, parallel);
        if capacity <= 8 || self.checks < 32 {
            batch(
                self.generator,
                &mut results,
                0,
                1,
                self.checks,
                self.shrinks,
                (&errors, self.errors),
                &random,
                check,
            );
        } else {
            scope(|scope| {
                let mut handles = Vec::with_capacity(parallel - 1);
                for offset in 0..parallel - 1 {
                    let check = &check;
                    let errors = &errors;
                    let seed = random.u64(..);
                    handles.push(scope.spawn(move || {
                        let mut results = Vec::with_capacity(capacity);
                        batch(
                            self.generator,
                            &mut results,
                            offset,
                            parallel,
                            self.checks,
                            self.shrinks,
                            (errors, self.errors),
                            &Rng::with_seed(seed),
                            check,
                        );
                        results
                    }));
                }

                batch(
                    self.generator,
                    &mut results,
                    parallel - 1,
                    parallel,
                    self.checks,
                    self.shrinks,
                    (&errors, self.errors),
                    &random,
                    &check,
                );

                for handle in handles {
                    results.extend(handle.join().into_iter().flatten());
                }
            });
        }
        results.into_iter()
    }
}

fn next<G: Generate + ?Sized, P: Prove, F: FnMut(&G::Item) -> P>(
    generator: &G,
    index: usize,
    count: usize,
    shrinks: Shrinks,
    random: &Rng,
    mut check: F,
) -> Result<G::Item, Error<G::Item, P>> {
    let mut state = State::from_iteration(index, count, Some(random.u64(..)));
    let (outer_item, mut outer_shrink) = generator.generate(&mut state);
    let outer_prove = check(&outer_item);
    if outer_prove.prove() {
        return Ok(outer_item);
    }

    let mut error = Error {
        state,
        index,
        count,
        original: (outer_item, outer_prove),
        shrinks: Shrinks {
            accept: 0,
            reject: 0,
        },
        shrunk: None,
    };

    while error.shrinks.reject < shrinks.reject && error.shrinks.accept < shrinks.accept {
        if let Some(inner_shrink) = outer_shrink.shrink() {
            let inner_item = inner_shrink.generate();
            let inner_prove = check(&inner_item);
            if inner_prove.prove() {
                error.shrinks.reject += 1;
            } else {
                error.shrinks.reject = 0;
                error.shrinks.accept += 1;
                error.shrunk = Some((inner_item, inner_prove));
                outer_shrink = inner_shrink;
            }
        } else {
            break;
        }
    }
    Err(error)
}

fn batch<G: Generate + ?Sized, P: Prove, F: Fn(&G::Item) -> P>(
    generator: &G,
    results: &mut Vec<Result<G::Item, Error<G::Item, P>>>,
    offset: usize,
    step: usize,
    count: usize,
    shrinks: Shrinks,
    errors: (&AtomicUsize, usize),
    random: &Rng,
    check: F,
) {
    for index in (offset..count).step_by(step) {
        match next(generator, index, count, shrinks, random, &check) {
            Ok(item) if errors.0.load(Ordering::Relaxed) < errors.1 => results.push(Ok(item)),
            Err(error) if errors.0.fetch_add(1, Ordering::Relaxed) < errors.1 => {
                results.push(Err(error))
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
