use crate::{
    Generate, Prove,
    check::{Check, Checker, Fail, Pass, Result},
};
use core::{
    any::type_name,
    cell::Cell,
    fmt::{self, Arguments},
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
) -> hook::Guard {
    checker.generate.items = verbose;
    checker.shrink.items = verbose;
    checker.shrink.errors = verbose;
    environment::update(checker);
    update(checker);
    Guard::new()
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

pub mod synchronous {
    use super::*;

    #[track_caller]
    pub fn default<G: Generate, U: FnOnce(&mut Checker<G>), P: Prove, C: Fn(G::Item) -> P>(
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
        with(
            generator,
            update,
            check,
            color,
            verbose,
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
        );
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
        with(
            generator,
            update,
            check,
            color,
            verbose,
            |prefix, pass| println!("{prefix} {pass:?}"),
            |prefix, fail| eprintln!("{prefix} {fail:?}"),
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
            |prefix, pass| {
                println!(
                    "{prefix} {{ type: {}, seed: {}, size: {} }}",
                    type_name::<G::Item>(),
                    pass.seed(),
                    pass.size(),
                )
            },
            |prefix, fail| {
                eprintln!(
                    "{prefix} {{ type: {}, seed: {}, size: {} }}",
                    type_name::<G::Item>(),
                    fail.seed(),
                    fail.size(),
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
        let colors = Colors::new(color);
        let _guard = prepare(&mut checker, update, verbose);
        for result in checker.checks(hook::silent(check)) {
            handle(result, &colors, &pass, &fail);
        }
    }
}

#[cfg(feature = "asynchronous")]
pub mod asynchronous {
    use super::*;
    use crate::check;
    use async_io::block_on;
    use core::future::Future;
    use futures_lite::StreamExt;

    #[track_caller]
    pub fn default<
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
    ) where
        G::Item: fmt::Debug,
        <P::Output as Prove>::Proof: fmt::Debug,
        <P::Output as Prove>::Error: fmt::Debug,
    {
        with(
            generator,
            update,
            check,
            color,
            verbose,
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

    #[track_caller]
    pub fn debug<
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
    ) where
        G::Item: fmt::Debug,
        <P::Output as Prove>::Proof: fmt::Debug,
        <P::Output as Prove>::Error: fmt::Debug,
    {
        with(
            generator,
            update,
            check,
            color,
            verbose,
            |prefix, pass| println!("{prefix} {pass:?}"),
            |prefix, fail| eprintln!("{prefix} {fail:?}"),
        )
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
        with(
            generator,
            update,
            check,
            color,
            verbose,
            |prefix, pass| {
                println!(
                    "{prefix} {{ type: {}, seed: {}, size: {} }}",
                    type_name::<G::Item>(),
                    pass.seed(),
                    pass.size(),
                )
            },
            |prefix, fail| {
                eprintln!(
                    "{prefix} {{ type: {}, seed: {}, size: {} }}",
                    type_name::<G::Item>(),
                    fail.seed(),
                    fail.size(),
                )
            },
        )
    }

    #[track_caller]
    fn with<
        G: Generate<Shrink: Unpin> + Unpin,
        U: FnOnce(&mut Checker<G, check::asynchronous::Run>),
        P: Future<Output: Prove<Error: Unpin> + Unpin>,
        C: Fn(G::Item) -> P + Unpin,
        WP: Fn(Arguments, Pass<G::Item, <P::Output as Prove>::Proof>),
        WF: Fn(Arguments, Fail<G::Item, <P::Output as Prove>::Error>),
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
        verbose: bool,
        pass: WP,
        fail: WF,
    ) {
        let mut checker = generator.checker().asynchronous();
        let colors = Colors::new(color);
        let _guard = prepare(&mut checker, update, verbose);
        let mut checks = checker.checks(hook::silent(check));
        while let Some(result) = block_on(checks.next()) {
            handle(result, &colors, &pass, &fail);
        }
    }
}

mod hook {
    use super::*;

    pub struct Guard;

    #[rustversion::since(1.81)]
    type Handle = Box<dyn Fn(&panic::PanicHookInfo) + 'static + Sync + Send>;
    #[rustversion::before(1.81)]
    type Handle = Box<dyn Fn(&panic::PanicInfo) + 'static + Sync + Send>;
    thread_local! { static HOOK: Cell<Option<Handle>> = const { Cell::new(None) }; }

    impl Guard {
        pub fn new() -> Self {
            begin();
            Self
        }
    }

    impl Drop for Guard {
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
