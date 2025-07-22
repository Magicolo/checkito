use crate::{
    Generate, Prove,
    check::{self, Check, Checker, Fail, Pass, Result},
};
use core::{
    any::type_name,
    cell::Cell,
    fmt::{self, Arguments},
    ops::{Deref, DerefMut},
    str::FromStr,
};
use hook::Guard;
use std::{env, panic};

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

fn prepare<G: Generate + ?Sized, R: ?Sized, U: FnOnce(&mut Checker<G, R>)>(
    checker: &mut Checker<G, R>,
    update: U,
    verbose: bool,
    color: bool,
) -> hook::Guard<Colors> {
    checker.generate.items = verbose;
    checker.shrink.items = verbose;
    checker.shrink.errors = verbose;
    environment::update(checker);
    update(checker);
    Guard::new(Colors::new(color))
}

fn handle<
    T,
    P: Prove,
    WP: Fn(Arguments, Pass<T, P::Proof>),
    WF: Fn(Arguments, Fail<T, P::Error>),
>(
    result: Result<T, P>,
    &Colors {
        red,
        green,
        yellow,
        dim,
        bold,
        reset,
    }: &Colors,
    pass: WP,
    fail: WF,
) {
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

fn handle_default<T: fmt::Debug, P: Prove<Proof: fmt::Debug, Error: fmt::Debug>>(
    result: Result<T, P>,
    colors: &Colors,
) {
    handle(
        result,
        colors,
        |prefix, pass| {
            println!(
                "{prefix} {{ item: {:?}, seed: {}, size: {}, proof: {:?} }}",
                &pass.item,
                pass.seed(),
                pass.size(),
                &pass.proof,
            )
        },
        |prefix, fail| {
            eprintln!(
                "{prefix} {{ item: {:?}, seed: {}, size: {}, message: \"{}\" }}",
                &fail.item,
                fail.seed(),
                fail.size(),
                fail.message(),
            )
        },
    )
}

fn handle_debug<T: fmt::Debug, P: Prove<Proof: fmt::Debug, Error: fmt::Debug>>(
    result: Result<T, P>,
    colors: &Colors,
) {
    handle(
        result,
        colors,
        |prefix, pass| println!("{prefix} {pass:?}"),
        |prefix, fail| eprintln!("{prefix} {fail:?}"),
    )
}

fn handle_minimal<T, P: Prove>(result: Result<T, P>, colors: &Colors) {
    handle(
        result,
        colors,
        |prefix, pass| {
            println!(
                "{prefix} {{ type: {}, seed: {}, size: {} }}",
                type_name::<T>(),
                pass.seed(),
                pass.size(),
            )
        },
        |prefix, fail| {
            eprintln!(
                "{prefix} {{ type: {}, seed: {}, size: {} }}",
                type_name::<T>(),
                fail.seed(),
                fail.size(),
            )
        },
    )
}

pub mod synchronous {
    use super::*;

    #[track_caller]
    pub fn default<
        G: Generate<Item: fmt::Debug>,
        U: FnOnce(&mut Checker<G, check::synchronous::Run>),
        P: Prove<Proof: fmt::Debug, Error: fmt::Debug>,
        C: Fn(G::Item) -> P,
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
        verbose: bool,
    ) {
        with(generator, update, check, color, verbose, handle_default)
    }

    #[track_caller]
    pub fn debug<G: Generate, U: FnOnce(&mut Checker<G>), P: Prove, C: Fn(G::Item) -> P>(
        generator: G,
        update: U,
        check: C,
        color: bool,
        verbose: bool,
    ) where
        G::Item: fmt::Debug,
        P::Proof: fmt::Debug,
        P::Error: fmt::Debug,
    {
        with(generator, update, check, color, verbose, handle_debug);
    }

    #[track_caller]
    pub fn minimal<G: Generate, U: FnOnce(&mut Checker<G>), P: Prove, C: Fn(G::Item) -> P>(
        generator: G,
        update: U,
        check: C,
        color: bool,
        verbose: bool,
    ) {
        with(generator, update, check, color, verbose, handle_minimal);
    }

    #[track_caller]
    fn with<
        G: Generate,
        U: FnOnce(&mut Checker<G>),
        P: Prove,
        C: Fn(G::Item) -> P,
        H: Fn(Result<G::Item, P>, &Colors),
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
        verbose: bool,
        handle: H,
    ) {
        let mut checker = generator.checker();
        let Guard(colors) = &prepare(&mut checker, update, verbose, color);
        checker
            .checks(hook::silent(check))
            .for_each(|result| handle(result, colors));
    }
}

#[cfg(feature = "asynchronous")]
pub mod asynchronous {
    use super::*;
    use crate::check;
    use core::future::Future;
    use futures_lite::{StreamExt, future::block_on};

    #[track_caller]
    pub fn default<
        G: Generate<Item: fmt::Debug, Shrink: Unpin> + Unpin,
        U: FnOnce(&mut Checker<G, check::asynchronous::Run>),
        P: Future<Output: Prove<Proof: fmt::Debug, Error: fmt::Debug + Unpin> + Unpin>,
        C: Fn(G::Item) -> P + Unpin,
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
        verbose: bool,
    ) {
        with(generator, update, check, color, verbose, handle_default)
    }

    #[track_caller]
    pub fn debugt<
        G: Generate<Item: fmt::Debug, Shrink: Unpin> + Unpin,
        U: FnOnce(&mut Checker<G, check::asynchronous::Run>),
        P: Future<Output: Prove<Proof: fmt::Debug, Error: fmt::Debug + Unpin> + Unpin>,
        C: Fn(G::Item) -> P + Unpin,
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
        verbose: bool,
    ) where
        G::Item: fmt::Debug,
        <P::Output as Prove>::Proof: fmt::Debug,
        <P::Output as Prove>::Error: fmt::Debug,
    {
        with(generator, update, check, color, verbose, handle_debug)
    }

    #[track_caller]
    pub fn minimal<
        G: Generate<Shrink: Unpin> + Unpin,
        U: FnOnce(&mut Checker<G, check::asynchronous::Run>),
        P: Future<Output: Prove<Error: Unpin> + Unpin>,
        C: Fn(G::Item) -> P + Unpin,
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
        verbose: bool,
    ) {
        with(generator, update, check, color, verbose, handle_minimal)
    }

    #[track_caller]
    fn with<
        G: Generate<Shrink: Unpin> + Unpin,
        U: FnOnce(&mut Checker<G, check::asynchronous::Run>),
        P: Future<Output: Prove<Error: Unpin> + Unpin>,
        C: Fn(G::Item) -> P + Unpin,
        H: Fn(Result<G::Item, P::Output>, &Colors),
    >(
        generator: G,
        update: U,
        check: C,
        verbose: bool,
        color: bool,
        handle: H,
    ) {
        let mut checker = generator.checker().asynchronous();
        let Guard(colors) = &prepare(&mut checker, update, verbose, color);
        block_on(
            checker
                // TODO: Is it possible to use `hook::silent` (adapted for futures) here?
                .checks(check)
                .for_each(|result| handle(result, colors)),
        );
    }
}

mod hook {
    use super::*;

    pub struct Guard<T: ?Sized>(pub T);

    #[rustversion::since(1.81)]
    type Handle = Box<dyn Fn(&panic::PanicHookInfo) + 'static + Sync + Send>;
    #[rustversion::before(1.81)]
    type Handle = Box<dyn Fn(&panic::PanicInfo) + 'static + Sync + Send>;
    thread_local! { static HOOK: Cell<Option<Handle>> = const { Cell::new(None) }; }

    impl<T> Guard<T> {
        pub fn new(state: T) -> Self {
            begin();
            Self(state)
        }
    }

    impl<T: ?Sized> Deref for Guard<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T: ?Sized> DerefMut for Guard<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl<T: ?Sized> AsRef<T> for Guard<T> {
        fn as_ref(&self) -> &T {
            &self.0
        }
    }

    impl<T: ?Sized> AsMut<T> for Guard<T> {
        fn as_mut(&mut self) -> &mut T {
            &mut self.0
        }
    }

    impl<T: ?Sized> Drop for Guard<T> {
        fn drop(&mut self) {
            end();
        }
    }

    pub fn begin() {
        HOOK.with(|cell| cell.set(Some(panic::take_hook())));
        panic::set_hook(Box::new(|panic| {
            HOOK.with(|cell| {
                if let Some(hook) = cell.replace(None) {
                    hook(panic);
                    cell.set(Some(hook));
                }
            });
        }));
    }

    pub fn silent<I, O>(function: impl Fn(I) -> O) -> impl Fn(I) -> O {
        move |input| {
            HOOK.with(|cell| {
                let hook = cell.replace(None);
                let output = function(input);
                cell.set(hook);
                output
            })
        }
    }

    pub fn end() {
        HOOK.with(|cell| {
            if let Some(hook) = cell.take() {
                panic::set_hook(hook);
            }
        });
    }

    pub fn panic() -> ! {
        end();
        panic!();
    }
}

mod environment {
    use super::*;

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

        pub fn update<G: ?Sized, R: ?Sized>(checker: &mut Checker<G, R>) {
            if let Some(value) = size() {
                checker.generate.sizes = (value..=value).into();
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

        pub fn update<G: ?Sized, R: ?Sized>(checker: &mut Checker<G, R>) {
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

    pub fn update<G: ?Sized, R: ?Sized>(checker: &mut Checker<G, R>) {
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
