use crate::{generate::State, prove::Prove, random, shrink::Shrink, Generate};
use core::{error, fmt, ops::Range, panic::AssertUnwindSafe, time::Duration};
use std::{borrow::Cow, panic::catch_unwind, time::Instant};

/// Bounds the shrinking process.
#[derive(Clone, Copy, Debug)]
pub struct Shrinks {
    /// Maximum number of successful attempts at reducing the 'size' of the input before aborting the shrinking process.
    /// Defaults to `usize::MAX`.
    pub accept: usize,
    /// Maximum number of failed attempts at reducing the 'size' of the input before aborting the shrinking process.
    /// Defaults to `usize::MAX`.
    pub reject: usize,
    /// Maximum time spent shrinking.
    /// Defaults to 30 seconds.
    pub duration: Duration,
}

/// The [`Checker`] structure holds a reference to a [`Generate`] instance and some configuration options for the checking and shrinking processes.
#[derive(Debug)]
pub struct Checker<'a, G: ?Sized> {
    /// A generator that will provide the values and shrinkers for the checking and shrinking processes.
    pub generator: &'a G,
    /// Whether or not the [`Checks`] iterator will yield items. When `false`, the iterator will only yield errors.
    /// Defaults to `true`.
    pub items: bool,
    /// Limits the shrinking process.
    /// Defaults to a duration limit of 30 seconds.
    pub shrinks: Shrinks,
    /// Seed for the random number generator used to generate random primitives.
    /// Defaults to a random value.
    pub seed: u64,
    /// Range of sizes that will be gradually traversed while generating values.
    /// Defaults to `0.0..1.0`.
    pub size: Range<f64>,
    /// Number of checks that will be performed.
    /// Defaults to `1000`.
    pub count: usize,
}

/// A structure representing a series of checks to be performed on a generator.
///
/// This structure is used to iterate over a sequence of checks, where each check
/// is performed on a generated item. It keeps track of the number of errors
/// encountered and the number of checks remaining.
#[derive(Debug)]
pub struct Checks<'a, G: ?Sized, F> {
    checker: Checker<'a, G>,
    items: bool,
    index: usize,
    count: usize,
    check: F,
}

pub trait Check: Generate {
    fn checker(&self) -> Checker<Self> {
        Checker::new(self, random::seed())
    }

    fn checks<P: Prove, F: FnMut(Self::Item) -> P>(
        &self,
        count: usize,
        check: F,
    ) -> Checks<Self, F> {
        let mut checker = self.checker();
        checker.count = count;
        checker.checks(check)
    }

    fn check<P: Prove, F: FnMut(Self::Item) -> P>(
        &self,
        count: usize,
        check: F,
    ) -> Result<(), Error<Self::Item, P>> {
        let mut checker = self.checker();
        checker.count = count;
        checker.items = false;
        for result in checker.checks(check) {
            result?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
/// An error produced by a check failure.
/// A check fails when a proof `P` is `false` for a given generated value.
pub struct Error<T, P> {
    /// The generator state that caused the error.
    pub original: T,
    pub shrunk: Option<T>,
    pub state: State,
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

pub const COUNT: usize = 1000;

impl<G: Generate + ?Sized> Check for G {}

impl<'a, G: Generate + ?Sized> Checker<'a, G> {
    pub(crate) fn new(generator: &'a G, seed: u64) -> Self {
        Self {
            generator,
            items: true,
            shrinks: Shrinks {
                accept: usize::MAX,
                reject: usize::MAX,
                duration: Duration::from_secs(30),
            },
            seed,
            size: 0.0..1.0,
            count: if generator.constant() { 1 } else { COUNT },
        }
    }
}

impl<G: ?Sized> Clone for Checker<'_, G> {
    fn clone(&self) -> Self {
        Self {
            generator: self.generator,
            items: self.items,
            shrinks: self.shrinks,
            seed: self.seed,
            count: self.count,
            size: self.size.clone(),
        }
    }
}

impl<G: ?Sized, F: Clone> Clone for Checks<'_, G, F> {
    fn clone(&self) -> Self {
        Self {
            checker: self.checker.clone(),
            check: self.check.clone(),
            items: self.items,
            index: self.index,
            count: self.count,
        }
    }
}

impl<'a, G: Generate + ?Sized> Checker<'a, G> {
    pub fn check<P: Prove, F: FnMut(G::Item) -> P>(
        &self,
        size: f64,
        check: F,
    ) -> Result<G::Item, Error<G::Item, P>> {
        next(
            self.generator,
            State::new(0, 1, size..size, self.seed),
            self.shrinks,
            check,
        )
        .map(|shrink| shrink.item())
    }

    pub fn checks<P: Prove, F: FnMut(G::Item) -> P>(&self, check: F) -> Checks<'a, G, F> {
        Checks {
            checker: self.clone(),
            items: self.items,
            count: self.count,
            check,
            index: 0,
        }
    }
}

#[cfg(feature = "parallel")]
impl<'a, G: Generate + ?Sized + Sync> Checker<'a, G>
where
    G::Item: Send,
{
    pub fn check_parallel<'b, P: Prove + Send + 'b, F: Fn(G::Item) -> P + Sync + 'b>(
        &'b self,
        count: usize,
        parallel: Option<usize>,
        check: F,
    ) -> impl Iterator<Item = Result<G::Item, Error<G::Item, P>>> + 'b {
        use std::{
            sync::{
                atomic::{AtomicUsize, Ordering},
                Mutex,
            },
            thread::{available_parallelism, scope},
        };

        type Results<G, P> =
            Mutex<Vec<Result<<G as Generate>::Item, Error<<G as Generate>::Item, P>>>>;

        const fn divide_ceiling(left: usize, right: usize) -> usize {
            let value = left / right;
            let remain = left % right;
            if remain > 0 && right > 0 {
                value + 1
            } else {
                value
            }
        }

        fn batch<G: Generate + ?Sized, P: Prove, F: Fn(G::Item) -> P>(
            generator: &G,
            results: &Results<G, P>,
            (offset, step, count): (usize, usize, usize),
            shrinks: Shrinks,
            errors: (&AtomicUsize, NonZeroUsize),
            (size, seed): (Range<f64>, u64),
            check: F,
        ) {
            for index in (offset..count).step_by(step) {
                let state = State::new(index, count, size.clone(), seed);
                match next(generator, state, shrinks, &check) {
                    Ok(shrink) if errors.0.load(Ordering::Relaxed) < errors.1.get() => {
                        results.lock().unwrap().push(Ok(shrink.item()))
                    }
                    Err(error) if errors.0.fetch_add(1, Ordering::Relaxed) < errors.1.get() => {
                        results.lock().unwrap().push(Err(error))
                    }
                    _ => break,
                }
            }
        }

        let parallel = match parallel {
            Some(parallel) => parallel.max(1),
            None => available_parallelism().map_or(1, NonZeroUsize::get),
        };
        let results = Mutex::new(Vec::with_capacity(count));
        let errors = AtomicUsize::new(0);
        let capacity = divide_ceiling(count, parallel);
        if capacity <= 8 || count < 32 {
            batch(
                self.generator,
                &results,
                (0, 1, count),
                self.shrinks,
                (&errors, self.errors),
                (self.size.clone(), self.seed),
                check,
            );
        } else {
            scope(|scope| {
                for offset in 1..parallel {
                    let check = &check;
                    let errors = &errors;
                    let results = &results;
                    let seed = self.seed;
                    let size = self.size.clone();
                    scope.spawn(move || {
                        batch(
                            self.generator,
                            results,
                            (offset, parallel, count),
                            self.shrinks,
                            (errors, self.errors),
                            (size, seed),
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
                    (self.size.clone(), self.seed),
                    &check,
                )
            });
        }
        results.into_inner().unwrap().into_iter()
    }
}

impl<G: Generate + ?Sized, P: Prove, F: FnMut(G::Item) -> P> Iterator for Checks<'_, G, F> {
    type Item = Result<G::Item, Error<G::Item, P>>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.count {
            let result = next(
                self.checker.generator,
                State::new(
                    self.index,
                    self.count,
                    self.checker.size.clone(),
                    self.checker.seed,
                ),
                self.checker.shrinks,
                &mut self.check,
            );
            self.index += 1;
            match result {
                Ok(shrink) if self.items => return Some(Ok(shrink.item())),
                Ok(_) => continue,
                Err(error) => return Some(Err(error)),
            }
        }
        None
    }
}

impl<T, P> Error<T, P> {
    pub fn item(&self) -> &T {
        self.shrunk.as_ref().unwrap_or(&self.original)
    }

    pub fn seed(&self) -> u64 {
        self.state.seed()
    }

    pub fn index(&self) -> usize {
        self.state.index()
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

fn handle<T, P: Prove, F: FnMut(T) -> P>(item: T, mut check: F) -> Option<Cause<P>> {
    let error = match catch_unwind(AssertUnwindSafe(|| check(item))) {
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

fn shrink<S: Shrink, P: Prove, F: FnMut(S::Item) -> P>(
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
        match handle(shrunk.item(), check) {
            Some(cause) => {
                *shrinker = shrunk;
                error.shrunk = Some(shrinker.item());
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

fn next<G: Generate + ?Sized, P: Prove, F: FnMut(G::Item) -> P>(
    generator: &G,
    mut state: State,
    shrinks: Shrinks,
    mut check: F,
) -> Result<G::Shrink, Error<G::Item, P>> {
    let mut shrinker = generator.generate(&mut state);
    let Some(cause) = handle(shrinker.item(), &mut check) else {
        return Ok(shrinker);
    };
    let mut error = Error {
        state,
        cause,
        original: shrinker.item(),
        shrunk: None,
        shrinks: Shrinks {
            accept: 0,
            reject: 0,
            duration: Duration::ZERO,
        },
    };
    while shrink(shrinks, &mut shrinker, &mut error, &mut check).is_some() {}
    Err(error)
}

impl<T: fmt::Debug, P: fmt::Debug> fmt::Display for Error<T, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<T: fmt::Debug, P: fmt::Debug> error::Error for Error<T, P> {}

#[doc(hidden)]
pub mod run {
    use super::{environment, Check, Checker, Error};
    use crate::{Generate, Prove};
    use core::{any::type_name, fmt};
    use std::panic;

    pub fn debug<
        G: Generate<Item: fmt::Debug>,
        U: FnOnce(&mut Checker<G>),
        P: Prove + fmt::Debug,
        C: Fn(G::Item) -> P,
    >(
        generator: G,
        update: U,
        check: C,
    ) {
        with(
            generator,
            true,
            update,
            check,
            |index, result| match result {
                Ok(item) => {
                    println!("\x1b[32mCHECK({})\x1b[0m: {:?}", index + 1, item);
                }
                Err(error) => {
                    eprintln!("\x1b[31mCHECK({})\x1b[0m: {error:?}", index + 1);
                    panic!();
                }
            },
        );
    }

    pub fn default<
        G: Generate<Item: fmt::Debug>,
        U: FnOnce(&mut Checker<G>),
        P: Prove + fmt::Debug,
        C: Fn(G::Item) -> P,
    >(
        generator: G,
        update: U,
        check: C,
    ) {
        with(generator, false, update, check, |_, result| {
            if let Err(error) = result {
                eprintln!();
                eprintln!(
                    "\x1b[31mCHECK({})\x1b[0m: {{ item: {:?}, seed: {}, message: \"{}\" }}",
                    error.index() + 1,
                    error.item(),
                    error.seed(),
                    error.message()
                );
                panic!();
            }
        });
    }

    pub fn minimal<
        G: Generate<Item: fmt::Debug>,
        U: FnOnce(&mut Checker<G>),
        P: Prove + fmt::Debug,
        C: Fn(G::Item) -> P,
    >(
        generator: G,
        update: U,
        check: C,
    ) {
        with(generator, false, update, check, |_, result| {
            if let Err(error) = result {
                eprintln!();
                eprintln!(
                    "\x1b[31mCHECK({})\x1b[0m: {{ type: {:?}, seed: {} }}",
                    error.index() + 1,
                    type_name::<G::Item>(),
                    error.seed()
                );
                panic!();
            }
        });
    }

    fn with<
        G: Generate,
        P: Prove,
        U: FnOnce(&mut Checker<G>),
        C: Fn(G::Item) -> P,
        H: Fn(usize, Result<G::Item, Error<G::Item, P>>),
    >(
        generator: G,
        debug: bool,
        update: U,
        check: C,
        handle: H,
    ) {
        let mut checker = generator.checker();
        environment::update(&mut checker);
        update(&mut checker);
        checker.items = debug;
        if !debug {
            panic::set_hook(Box::new(|_| {}));
        }
        for (index, result) in checker.checks(check).enumerate() {
            handle(index, result);
        }
    }
}

#[doc(hidden)]
pub mod environment {
    use super::Checker;
    use std::{env, str::FromStr, time::Duration};

    pub fn count() -> Option<usize> {
        parse("CHECKITO_COUNT")
    }

    pub fn size() -> Option<f64> {
        parse("CHECKITO_SIZE")
    }

    pub fn seed() -> Option<u64> {
        parse("CHECKITO_SEED")
    }

    pub fn accept() -> Option<usize> {
        parse("CHECKITO_ACCEPT")
    }

    pub fn reject() -> Option<usize> {
        parse("CHECKITO_REJECT")
    }

    pub fn duration() -> Option<Duration> {
        parse("CHECKITO_DURATION").map(Duration::from_secs_f64)
    }

    pub fn update<G>(checker: &mut Checker<'_, G>) {
        if let Some(value) = size() {
            checker.size = value..value;
        }
        if let Some(value) = count() {
            checker.count = value;
        }
        if let Some(value) = seed() {
            checker.seed = value;
        }
        if let Some(value) = accept() {
            checker.shrinks.accept = value;
        }
        if let Some(value) = reject() {
            checker.shrinks.reject = value;
        }
        if let Some(value) = duration() {
            checker.shrinks.duration = value;
        }
    }

    fn parse<T: FromStr>(key: &str) -> Option<T> {
        match env::var(key) {
            Ok(value) => value.parse().ok(),
            Err(_) => None,
        }
    }
}
