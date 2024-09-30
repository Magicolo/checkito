use crate::{generate::State, prove::Prove, random, shrink::Shrink, Generate};
use core::{error, fmt, mem::replace, ops::Range, panic::AssertUnwindSafe};
use std::{borrow::Cow, panic::catch_unwind};

/// Bounds the generation process.
#[derive(Clone, Debug)]
pub struct Generates {
    /// Seed for the random number generator used to generate random primitives.
    /// Defaults to a random value.
    pub seed: u64,
    /// Range of sizes that will be gradually traversed while generating values.
    /// Defaults to `0.0..1.0`.
    pub size: Range<f64>,
    /// Maximum number of items that will be generated.
    /// Defaults to `1000`.
    pub count: usize,
    /// Whether or not the [`Checks`] iterator will yield generation items.
    /// Defaults to `true`.
    pub items: bool,
}

/// Bounds the shrinking process.
#[derive(Clone, Debug)]
pub struct Shrinks {
    /// Maximum number of attempts at shrinking an item that has failed a check.
    /// Defaults to `usize::MAX`.
    pub count: usize,
    /// Whether or not the [`Checks`] iterator will yield shrinking items.
    /// Defaults to `true`.
    pub items: bool,
    /// Whether or not the [`Checks`] iterator will yield shrinking errors.
    /// Defaults to `true`.
    pub errors: bool,
}

/// The [`Checker`] structure holds a reference to a [`Generate`] instance and some configuration options for the checking and shrinking processes.
#[derive(Debug)]
pub struct Checker<'a, G: ?Sized> {
    /// A generator that will generate items and their shrinkers for checking a property.
    generator: &'a G,
    /// Bounds the generation process.
    pub generate: Generates,
    /// Bounds the shrinking process.
    pub shrink: Shrinks,
}

/// A structure representing a series of checks to be performed on a generator.
///
/// This structure is used to iterate over a sequence of checks, where each check
/// is performed on a generated item. It keeps track of the number of errors
/// encountered and the number of checks remaining.
pub struct Checks<'a, G: Generate + ?Sized, E, F> {
    checker: Checker<'a, G>,
    machine: Machine<G::Shrink, E>,
    check: F,
}

enum Machine<S, E> {
    Generate {
        index: usize,
    },
    Shrink {
        indices: (usize, usize),
        state: State,
        shrinker: S,
        cause: Cause<E>,
    },
    Done,
}

pub trait Check: Generate {
    fn checker(&self) -> Checker<Self> {
        Checker::new(self, random::seed())
    }

    fn checks<P: Prove, F: FnMut(Self::Item) -> P>(&self, check: F) -> Checks<Self, P::Error, F> {
        self.checker().checks(check)
    }

    fn check<P: Prove, F: FnMut(Self::Item) -> P>(
        &self,
        check: F,
    ) -> Result<(), Error<Self::Item, P::Error>> {
        let mut checker = self.checker();
        checker.generate.items = false;
        checker.shrink.items = false;
        checker.shrink.errors = false;
        match checker.checks(check).last() {
            Some(Ok(_)) | None => Ok(()),
            Some(Err(error)) => Err(error),
        }
    }
}

#[derive(Clone, Debug)]
/// An item that represents a successful check.
pub struct Item<T, P> {
    pub item: T,
    pub proof: P,
    pub generates: usize,
    pub shrinks: usize,
    pub shrink: bool,
    /// The generator state that produced the item.
    pub state: State,
}

#[derive(Clone, Debug)]
/// An error that represents a failed check.
pub struct Error<T, E> {
    pub item: T,
    pub cause: Cause<E>,
    pub generates: usize,
    pub shrinks: usize,
    pub shrink: bool,
    /// The generator state that caused the error.
    pub state: State,
}

/// The cause of a check failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cause<E> {
    /// A `Disprove` cause is a value that, when checked, returns a value of type `P`
    /// that does not satisfy the property.
    Disprove(E),
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
            generate: Generates {
                items: true,
                count: if generator.constant() { 1 } else { COUNT },
                seed,
                size: 0.0..1.0,
            },
            shrink: Shrinks {
                count: usize::MAX,
                items: true,
                errors: true,
            },
        }
    }
}

impl<G: ?Sized> Clone for Checker<'_, G> {
    fn clone(&self) -> Self {
        Self {
            generator: self.generator,
            generate: self.generate.clone(),
            shrink: self.shrink.clone(),
        }
    }
}

impl<'a, G: Generate + ?Sized> Checker<'a, G> {
    pub fn checks<P: Prove, F: FnMut(G::Item) -> P>(&self, check: F) -> Checks<'a, G, P::Error, F> {
        Checks {
            checker: self.clone(),
            machine: Machine::Generate { index: 0 },
            check,
        }
    }
}

impl<G: Generate + ?Sized, P: Prove, F: FnMut(G::Item) -> P> Iterator
    for Checks<'_, G, P::Error, F>
{
    type Item = Result<Item<G::Item, P::Proof>, Error<G::Item, P::Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match replace(&mut self.machine, Machine::Done) {
                Machine::Generate { index } if index >= self.checker.generate.count => break None,
                Machine::Generate { index } => {
                    let mut state = State::new(
                        index,
                        self.checker.generate.count,
                        self.checker.generate.size.clone(),
                        self.checker.generate.seed,
                    );
                    let shrinker = self.checker.generator.generate(&mut state);
                    let result = handle(shrinker.item(), &mut self.check);
                    match result {
                        Ok(proof) => {
                            self.machine = Machine::Generate { index: index + 1 };
                            if self.checker.generate.items {
                                break Some(Ok(Item {
                                    item: shrinker.item(),
                                    generates: index,
                                    shrinks: 0,
                                    shrink: false,
                                    proof,
                                    state,
                                }));
                            }
                        }
                        Err(cause) => {
                            self.machine = Machine::Shrink {
                                indices: (index, 0),
                                state,
                                shrinker,
                                cause,
                            };
                        }
                    }
                }
                Machine::Shrink {
                    indices,
                    state,
                    mut shrinker,
                    cause,
                } => {
                    if indices.1 >= self.checker.shrink.count {
                        break Some(Err(Error {
                            item: shrinker.item(),
                            generates: indices.0,
                            shrinks: indices.1,
                            shrink: false,
                            state,
                            cause,
                        }));
                    }

                    let new_shrinker = match shrinker.shrink() {
                        Some(shrinker) => shrinker,
                        None => {
                            break Some(Err(Error {
                                item: shrinker.item(),
                                generates: indices.0,
                                shrinks: indices.1,
                                shrink: false,
                                state,
                                cause,
                            }));
                        }
                    };
                    let result = handle(new_shrinker.item(), &mut self.check);
                    match result {
                        Ok(proof) => {
                            self.machine = Machine::Shrink {
                                indices: (indices.0, indices.1 + 1),
                                state: state.clone(),
                                shrinker,
                                cause,
                            };
                            if self.checker.shrink.items {
                                break Some(Ok(Item {
                                    item: new_shrinker.item(),
                                    generates: indices.0,
                                    shrinks: indices.1,
                                    shrink: true,
                                    proof,
                                    state,
                                }));
                            }
                        }
                        Err(new_cause) => {
                            self.machine = Machine::Shrink {
                                indices: (indices.0, indices.1 + 1),
                                state: state.clone(),
                                shrinker: new_shrinker,
                                cause: new_cause,
                            };
                            if self.checker.shrink.errors {
                                break Some(Err(Error {
                                    item: shrinker.item(),
                                    generates: indices.0,
                                    shrinks: indices.1,
                                    shrink: true,
                                    cause,
                                    state,
                                }));
                            }
                        }
                    }
                }
                _ => break None,
            }
        }
    }
}

impl<T, P> Item<T, P> {
    pub fn seed(&self) -> u64 {
        self.state.seed()
    }

    pub fn size(&self) -> f64 {
        self.state.size()
    }
}

impl<T, P> Error<T, P> {
    pub fn seed(&self) -> u64 {
        self.state.seed()
    }

    pub fn size(&self) -> f64 {
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

fn handle<T, P: Prove, F: FnMut(T) -> P>(
    item: T,
    mut check: F,
) -> Result<P::Proof, Cause<P::Error>> {
    let error = match catch_unwind(AssertUnwindSafe(move || check(item))) {
        Ok(prove) => match prove.prove() {
            Ok(ok) => return Ok(ok),
            Err(error) => return Err(Cause::Disprove(error)),
        },
        Err(error) => error,
    };
    let error = match error.downcast::<&'static str>() {
        Ok(error) => return Err(Cause::Panic(Some(Cow::Borrowed(*error)))),
        Err(error) => error,
    };
    let error = match error.downcast::<String>() {
        Ok(error) => return Err(Cause::Panic(Some(Cow::Owned(*error)))),
        Err(error) => error,
    };
    let error = match error.downcast::<Box<str>>() {
        Ok(error) => return Err(Cause::Panic(Some(Cow::Owned(error.to_string())))),
        Err(error) => error,
    };
    match error.downcast::<Cow<'static, str>>() {
        Ok(error) => Err(Cause::Panic(Some(*error))),
        Err(_) => Err(Cause::Panic(None)),
    }
}

impl<T: fmt::Debug, E: fmt::Debug> fmt::Display for Error<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<T: fmt::Debug, E: fmt::Debug> error::Error for Error<T, E> {}

#[doc(hidden)]
pub mod help {
    use super::{environment, Check, Checker, Error, Item};
    use crate::{Generate, Prove};
    use core::{
        any::type_name,
        fmt::{self, Arguments},
        ops::Range,
        time::Duration,
    };
    use std::panic;

    pub trait IntoRange<T> {
        fn range(self) -> Range<T>;
    }

    pub trait IntoDuration {
        fn duration(self) -> Duration;
    }

    struct Colors {
        red: &'static str,
        green: &'static str,
        yellow: &'static str,
        dim: &'static str,
        bold: &'static str,
        reset: &'static str,
    }

    impl Colors {
        pub const fn new(color: bool) -> Self {
            if color {
                Self {
                    red: "\x1b[31m",
                    green: "\x1b[32m",
                    yellow: "\x1b[33m",
                    bold: "\x1b[1m",
                    dim: "\x1b[2m",
                    reset: "\x1b[0m",
                }
            } else {
                Self {
                    red: "",
                    green: "",
                    yellow: "",
                    bold: "",
                    dim: "",
                    reset: "",
                }
            }
        }
    }

    #[track_caller]
    pub fn default<
        G: Generate<Item: fmt::Debug>,
        U: FnOnce(&mut Checker<G>),
        P: Prove<Proof: fmt::Debug, Error: fmt::Debug>,
        C: Fn(G::Item) -> P,
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
        verbose: bool,
    ) {
        with(
            generator,
            update,
            check,
            color,
            verbose,
            |prefix, item| {
                if item.shrink {
                    println!(
                        "{prefix} {{ item: {:?}, proof: {:?} }}",
                        &item.item, &item.proof,
                    )
                } else {
                    println!(
                        "{prefix} {{ item: {:?}, size: {}, proof: {:?} }}",
                        &item.item,
                        item.size(),
                        &item.proof,
                    )
                }
            },
            |prefix, error| {
                if error.shrink {
                    eprintln!(
                        "{prefix} {{ item: {:?}, message: \"{}\" }}",
                        &error.item,
                        error.message(),
                    )
                } else {
                    eprintln!(
                        "{prefix} {{ item: {:?}, seed: {}, size: {}, message: \"{}\" }}",
                        &error.item,
                        error.seed(),
                        error.size(),
                        error.message(),
                    )
                }
            },
        );
    }

    #[track_caller]
    pub fn debug<
        G: Generate<Item: fmt::Debug>,
        U: FnOnce(&mut Checker<G>),
        P: Prove<Proof: fmt::Debug, Error: fmt::Debug>,
        C: Fn(G::Item) -> P,
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
        verbose: bool,
    ) {
        with(
            generator,
            update,
            check,
            color,
            verbose,
            |prefix, item| println!("{prefix} {item:?}"),
            |prefix, error| eprintln!("{prefix} {error:?}"),
        );
    }

    #[track_caller]
    pub fn minimal<G: Generate, U: FnOnce(&mut Checker<G>), P: Prove, C: Fn(G::Item) -> P>(
        generator: G,
        update: U,
        check: C,
        color: bool,
        verbose: bool,
    ) {
        with(
            generator,
            update,
            check,
            color,
            verbose,
            |prefix, item| {
                if item.shrink {
                    println!("{prefix}")
                } else {
                    println!(
                        "{prefix} {{ type: {}, seed: {}, size: {} }}",
                        type_name::<G::Item>(),
                        item.seed(),
                        item.size(),
                    )
                }
            },
            |prefix, error| {
                if error.shrink {
                    eprintln!("{prefix}")
                } else {
                    eprintln!(
                        "{prefix} {{ type: {}, seed: {}, size: {} }}",
                        type_name::<G::Item>(),
                        error.seed(),
                        error.size(),
                    )
                }
            },
        );
    }

    #[track_caller]
    fn with<
        G: Generate,
        U: FnOnce(&mut Checker<G>),
        P: Prove,
        C: Fn(G::Item) -> P,
        I: Fn(Arguments, Item<G::Item, P::Proof>),
        E: Fn(Arguments, Error<G::Item, P::Error>),
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
        verbose: bool,
        item: I,
        error: E,
    ) {
        let mut checker = generator.checker();
        checker.generate.items = verbose;
        checker.shrink.items = verbose;
        checker.shrink.errors = verbose;
        environment::update(&mut checker);
        (update)(&mut checker);
        let hook = panic::take_hook();
        panic::set_hook(Box::new(|_| {}));
        let Colors {
            red,
            green,
            yellow,
            dim,
            bold,
            reset,
        } = Colors::new(color);
        for result in checker.checks(check) {
            match result {
                Ok(
                    value @ Item {
                        shrink: false,
                        generates,
                        ..
                    },
                ) => item(format_args!("{green}PASS({generates}){reset}"), value),
                Ok(
                    value @ Item {
                        shrink: true,
                        shrinks,
                        ..
                    },
                ) => item(
                    format_args!("{dim}{yellow}SHRINK({shrinks}, {green}PASS{yellow}){reset}"),
                    value,
                ),
                Err(
                    value @ Error {
                        shrink: true,
                        shrinks,
                        ..
                    },
                ) => error(
                    format_args!("{yellow}SHRUNK({shrinks}, {red}FAIL{yellow}){reset}"),
                    value,
                ),
                Err(
                    value @ Error {
                        shrink: false,
                        generates,
                        shrinks,
                        ..
                    },
                ) => {
                    error(
                        format_args!("{bold}{red}FAIL({generates}, {shrinks}){reset}"),
                        value,
                    );
                    panic::set_hook(hook);
                    panic!();
                }
            }
        }
        panic::set_hook(hook);
    }

    impl<T> IntoRange<T> for Range<T> {
        fn range(self) -> Range<T> {
            self
        }
    }

    macro_rules! range {
        ($($from: ty),*) => {
            $(
                impl IntoRange<$from> for $from {
                    fn range(self) -> Range<$from> {
                        self..self
                    }
                }
            )*
        };
    }
    range!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64, char, bool);

    impl IntoDuration for f32 {
        fn duration(self) -> Duration {
            Duration::from_secs_f32(self)
        }
    }

    impl IntoDuration for f64 {
        fn duration(self) -> Duration {
            Duration::from_secs_f64(self)
        }
    }

    macro_rules! duration {
        ($($from: ty),*) => {
            $(
                impl IntoDuration for $from {
                    fn duration(self) -> Duration {
                        Duration::from_secs(self as _)
                    }
                }
            )*
        };
    }
    duration!(u8, u16, u32, u64, u128, usize);
}

#[doc(hidden)]
pub mod environment {
    use super::Checker;
    use core::str::FromStr;
    use std::env;

    pub mod generate {
        use super::*;

        pub fn count() -> Option<usize> {
            parse("CHECKITO_GENERATE_COUNT")
        }

        pub fn size() -> Option<f64> {
            parse("CHECKITO_GENERATE_SIZE")
        }

        pub fn seed() -> Option<u64> {
            parse("CHECKITO_GENERATE_SEED")
        }

        pub fn items() -> Option<bool> {
            parse("CHECKITO_GENERATE_ITEMS")
        }

        pub fn update<G>(checker: &mut Checker<'_, G>) {
            if let Some(value) = size() {
                checker.generate.size = value..value;
            }
            if let Some(value) = count() {
                checker.generate.count = value;
            }
            if let Some(value) = seed() {
                checker.generate.seed = value;
            }
            if let Some(value) = items() {
                checker.generate.items = value;
            }
        }
    }

    pub mod shrink {
        use super::*;

        pub fn count() -> Option<usize> {
            parse("CHECKITO_SHRINK_COUNT")
        }

        pub fn items() -> Option<bool> {
            parse("CHECKITO_SHRINK_ITEMS")
        }

        pub fn errors() -> Option<bool> {
            parse("CHECKITO_SHRINK_ERRORS")
        }

        pub fn update<G>(checker: &mut Checker<'_, G>) {
            if let Some(value) = count() {
                checker.shrink.count = value;
            }
            if let Some(value) = items() {
                checker.shrink.items = value;
            }
            if let Some(value) = errors() {
                checker.shrink.errors = value;
            }
        }
    }

    pub fn update<G>(checker: &mut Checker<'_, G>) {
        generate::update(checker);
        shrink::update(checker);
    }

    fn parse<T: FromStr>(key: &str) -> Option<T> {
        match env::var(key) {
            Ok(value) => value.parse().ok(),
            Err(_) => None,
        }
    }
}
