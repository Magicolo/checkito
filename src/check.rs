use crate::{generate::State, prove::Prove, random, shrink::Shrink, Generate};
use core::{error, fmt, mem::replace, ops::Range, panic::AssertUnwindSafe};
use std::{any::Any, borrow::Cow, panic::catch_unwind, result};

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
    ) -> Option<Fail<Self::Item, P::Error>> {
        let mut checker = self.checker();
        checker.generate.items = false;
        checker.shrink.items = false;
        checker.shrink.errors = false;
        match checker.checks(check).last()? {
            Result::Pass(_) => None,
            Result::Fail(fail) => Some(fail),
            Result::Shrink(_) | Result::Shrunk(_) => {
                unreachable!("it is invalid for the `Checks` iterator to end on a shrinking result")
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Result<T, P: Prove> {
    Pass(Pass<T, P::Proof>),
    Shrink(Pass<T, P::Proof>),
    Shrunk(Fail<T, P::Error>),
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
    type Item = Result<G::Item, P>;

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
                                break Some(Result::Pass(Pass {
                                    item: shrinker.item(),
                                    generates: index,
                                    shrinks: 0,
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
                        break Some(Result::Fail(Fail {
                            item: shrinker.item(),
                            generates: indices.0,
                            shrinks: indices.1,
                            state,
                            cause,
                        }));
                    }

                    let new = match shrinker.shrink() {
                        Some(shrinker) => shrinker,
                        None => {
                            break Some(Result::Fail(Fail {
                                item: shrinker.item(),
                                generates: indices.0,
                                shrinks: indices.1,
                                state,
                                cause,
                            }));
                        }
                    };
                    let result = handle(new.item(), &mut self.check);
                    match result {
                        Ok(proof) => {
                            self.machine = Machine::Shrink {
                                indices: (indices.0, indices.1 + 1),
                                state: state.clone(),
                                shrinker,
                                cause,
                            };
                            if self.checker.shrink.items {
                                break Some(Result::Shrink(Pass {
                                    item: new.item(),
                                    generates: indices.0,
                                    shrinks: indices.1,
                                    proof,
                                    state,
                                }));
                            }
                        }
                        Err(new_cause) => {
                            self.machine = Machine::Shrink {
                                indices: (indices.0, indices.1 + 1),
                                state: state.clone(),
                                shrinker: new,
                                cause: new_cause,
                            };
                            if self.checker.shrink.errors {
                                break Some(Result::Shrunk(Fail {
                                    item: shrinker.item(),
                                    generates: indices.0,
                                    shrinks: indices.1,
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

    pub const fn item(&self) -> &T {
        match self {
            Result::Pass(pass) | Result::Shrink(pass) => &pass.item,
            Result::Fail(fail) | Result::Shrunk(fail) => &fail.item,
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

impl<T: fmt::Debug, E: fmt::Debug> fmt::Display for Fail<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<T: fmt::Debug, E: fmt::Debug> error::Error for Fail<T, E> {}

#[doc(hidden)]
pub mod help {
    use super::{environment, hook, Check, Checker, Fail, Pass, Result};
    use crate::{Generate, Prove};
    use core::{
        any::type_name,
        fmt::{self, Arguments},
        ops::Range,
        time::Duration,
    };

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
                println!(
                    "{prefix} {{ item: {:?}, size: {}, proof: {:?} }}",
                    &item.item,
                    item.size(),
                    &item.proof,
                )
            },
            |prefix, error| {
                eprintln!(
                    "{prefix} {{ item: {:?}, seed: {}, size: {}, message: \"{}\" }}",
                    &error.item,
                    error.seed(),
                    error.size(),
                    error.message(),
                )
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
                println!(
                    "{prefix} {{ type: {}, seed: {}, size: {} }}",
                    type_name::<G::Item>(),
                    item.seed(),
                    item.size(),
                )
            },
            |prefix, error| {
                eprintln!(
                    "{prefix} {{ type: {}, seed: {}, size: {} }}",
                    type_name::<G::Item>(),
                    error.seed(),
                    error.size(),
                )
            },
        );
    }

    #[track_caller]
    fn with<
        G: Generate,
        U: FnOnce(&mut Checker<G>),
        P: Prove,
        C: Fn(G::Item) -> P,
        WP: Fn(Arguments, Pass<G::Item, P::Proof>),
        WF: Fn(Arguments, Fail<G::Item, P::Error>),
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
        verbose: bool,
        pass: WP,
        fail: WF,
    ) {
        let mut checker = generator.checker();
        checker.generate.items = verbose;
        checker.shrink.items = verbose;
        checker.shrink.errors = verbose;
        environment::update(&mut checker);
        (update)(&mut checker);
        let Colors {
            red,
            green,
            yellow,
            dim,
            bold,
            reset,
        } = Colors::new(color);

        hook::reserve();
        for result in checker.checks(hook::wrap(check)) {
            match result {
                Result::Pass(value @ Pass { generates, .. }) => {
                    pass(format_args!("{green}PASS({generates}){reset}"), value)
                }
                Result::Shrink(value @ Pass { shrinks, .. }) => pass(
                    format_args!("{dim}{yellow}SHRINK({shrinks}, {green}PASS{yellow}){reset}"),
                    value,
                ),
                Result::Shrunk(value @ Fail { shrinks, .. }) => fail(
                    format_args!("{yellow}SHRUNK({shrinks}, {red}FAIL{yellow}){reset}"),
                    value,
                ),
                Result::Fail(
                    value @ Fail {
                        generates, shrinks, ..
                    },
                ) => {
                    fail(
                        format_args!("{bold}{red}FAIL({generates}, {shrinks}){reset}"),
                        value,
                    );
                    hook::panic();
                }
            }
        }
        hook::release();
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

mod hook {
    use core::cell::Cell;
    use std::panic::{self, PanicHookInfo};

    type Handle = Box<dyn Fn(&PanicHookInfo) + 'static + Sync + Send>;
    thread_local! { static HOOK: Cell<Option<Handle>> = const { Cell::new(None) }; }

    pub fn reserve() {
        HOOK.with(|cell| cell.set(Some(panic::take_hook())));
        panic::set_hook(Box::new(handle));
    }

    pub fn wrap<I, O>(function: impl Fn(I) -> O) -> impl Fn(I) -> O {
        move |input| {
            HOOK.with(|cell| {
                let hook = cell.replace(None);
                let output = function(input);
                cell.set(hook);
                output
            })
        }
    }

    pub fn release() {
        HOOK.with(|cell| {
            if let Some(hook) = cell.take() {
                panic::set_hook(hook);
            }
        });
    }

    pub fn panic() -> ! {
        release();
        panic!();
    }

    fn handle(panic: &PanicHookInfo) {
        HOOK.with(|cell| {
            if let Some(hook) = cell.replace(None) {
                hook(panic);
                cell.set(Some(hook));
            }
        });
    }
}

mod environment {
    use super::Checker;
    use core::str::FromStr;
    use std::env;

    mod generate {
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

    mod shrink {
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
