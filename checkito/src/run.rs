use crate::{
    Generate, Prove,
    check::{self, Check, Checker, Fail, Pass, Result},
};
use core::{
    any::type_name,
    cell::Cell,
    fmt::{self, Arguments},
};
use std::panic;

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

fn prepare<G: Generate + ?Sized, R, U: FnOnce(&mut Checker<G, R>)>(
    checker: &mut Checker<G, R>,
    update: U,
    color: bool,
) -> Colors {
    checker.environment();
    update(checker);
    Colors::new(color)
}

fn handle<
    T,
    P: Prove,
    WP: Fn(Arguments, &Pass<T, P::Proof>),
    WF: Fn(Arguments, &Fail<T, P::Error>),
>(
    result: &Result<T, P>,
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

fn print_default<T: fmt::Debug, P: Prove<Proof: fmt::Debug, Error: fmt::Debug>>(
    result: &Result<T, P>,
    colors: &Colors,
) {
    handle(
        &result,
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

fn print_debug<T: fmt::Debug, P: Prove<Proof: fmt::Debug, Error: fmt::Debug>>(
    result: &Result<T, P>,
    colors: &Colors,
) {
    handle(
        result,
        colors,
        |prefix, pass| println!("{prefix} {pass:?}"),
        |prefix, fail| eprintln!("{prefix} {fail:?}"),
    )
}

fn print_minimal<T, P: Prove>(result: &Result<T, P>, colors: &Colors) {
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

    type Run = check::synchronous::Run;

    #[track_caller]
    pub fn default<
        G: Generate<Item: fmt::Debug>,
        U: FnOnce(&mut Checker<G, Run>),
        P: Prove<Proof: fmt::Debug, Error: fmt::Debug>,
        C: Fn(G::Item) -> P,
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
    ) {
        with(generator, update, check, color, print_default)
    }

    #[track_caller]
    pub fn debug<
        G: Generate<Item: fmt::Debug>,
        U: FnOnce(&mut Checker<G, Run>),
        P: Prove<Proof: fmt::Debug, Error: fmt::Debug>,
        C: Fn(G::Item) -> P,
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
    ) {
        with(generator, update, check, color, print_debug);
    }

    #[track_caller]
    pub fn minimal<G: Generate, U: FnOnce(&mut Checker<G, Run>), P: Prove, C: Fn(G::Item) -> P>(
        generator: G,
        update: U,
        check: C,
        color: bool,
    ) {
        with(generator, update, check, color, print_minimal);
    }

    #[track_caller]
    fn with<
        G: Generate,
        U: FnOnce(&mut Checker<G, Run>),
        P: Prove,
        C: Fn(G::Item) -> P,
        H: Fn(&Result<G::Item, P>, &Colors),
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
        handle: H,
    ) {
        let mut checker = generator.checker();
        let colors = prepare(&mut checker, update, color);
        let guard = hook::capture();
        checker
            .checks(move |input| {
                let guard = hook::quiet();
                let output = check(input);
                drop(guard);
                output
            })
            .for_each(|result| handle(&result, &colors));
        drop(guard);
    }
}

#[cfg(feature = "asynchronous")]
pub mod asynchronous {
    use super::*;
    use crate::check;
    use core::future::Future;
    use futures_lite::{StreamExt, future::block_on};

    type Run = check::asynchronous::Run;

    #[track_caller]
    pub fn default<
        G: Generate<Item: fmt::Debug>,
        U: FnOnce(&mut Checker<G, Run>),
        P: Future<Output: Prove<Proof: fmt::Debug, Error: fmt::Debug>>,
        C: Fn(G::Item) -> P,
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
    ) {
        with(generator, update, check, color, print_default)
    }

    #[track_caller]
    pub fn debug<
        G: Generate<Item: fmt::Debug>,
        U: FnOnce(&mut Checker<G, Run>),
        P: Future<Output: Prove<Proof: fmt::Debug, Error: fmt::Debug>>,
        C: Fn(G::Item) -> P,
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
    ) {
        with(generator, update, check, color, print_debug)
    }

    #[track_caller]
    pub fn minimal<
        G: Generate,
        U: FnOnce(&mut Checker<G, Run>),
        P: Future<Output: Prove>,
        C: Fn(G::Item) -> P,
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
    ) {
        with(generator, update, check, color, print_minimal)
    }

    #[track_caller]
    fn with<
        G: Generate,
        U: FnOnce(&mut Checker<G, Run>),
        P: Future<Output: Prove>,
        C: Fn(G::Item) -> P,
        H: Fn(&Result<G::Item, P::Output>, &Colors),
    >(
        generator: G,
        update: U,
        check: C,
        color: bool,
        handle: H,
    ) {
        let mut checker = generator.checker().asynchronous(None);
        let colors = prepare(&mut checker, update, color);
        let guard = hook::capture();
        let check = &check;
        block_on(
            checker
                .checks(move |item| async move {
                    let guard = hook::quiet();
                    let proof = check(item).await;
                    drop(guard);
                    proof
                })
                .for_each(|result| handle(&result, &colors)),
        );
        drop(guard);
    }
}

mod hook {
    use super::*;

    pub(crate) struct EndGuard(());

    pub(crate) struct RestoreGuard(Option<Handle>);

    type Handle = Box<dyn Fn(&panic::PanicHookInfo) + 'static + Sync + Send>;
    thread_local! { static HOOK: Cell<Option<Handle>> = const { Cell::new(None) }; }

    impl Drop for EndGuard {
        fn drop(&mut self) {
            end();
        }
    }

    impl Drop for RestoreGuard {
        fn drop(&mut self) {
            HOOK.set(self.0.take());
        }
    }

    pub fn capture() -> EndGuard {
        begin();
        EndGuard(())
    }

    pub fn panic() -> ! {
        end();
        panic!();
    }

    pub fn quiet() -> RestoreGuard {
        RestoreGuard(HOOK.take())
    }

    fn begin() {
        HOOK.set(Some(panic::take_hook()));
        panic::set_hook(Box::new(|panic| {
            let guard = quiet();
            if let Some(hook) = guard.0.as_ref() {
                hook(panic);
            }
            drop(guard);
        }));
    }

    fn end() {
        if let Some(hook) = HOOK.take() {
            panic::set_hook(hook);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn silent_restores_hook_after_panicking_closure() {
            let _guard = capture();
            let _result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                let _guard = hook::quiet();
                panic!("boom");
            }));
            assert!(HOOK.take().is_some());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::FullGenerate;

    #[test]
    fn prepare_applies_color_and_verbose_independently() {
        let mut checker = bool::generator().checker();

        {
            checker.verbose(true);
            let guard = prepare(&mut checker, |_| {}, false);
            assert_eq!(guard.green, "");
            assert!(checker.generate.items);
            assert!(checker.shrink.items);
            assert!(checker.shrink.errors);
        }

        {
            checker.verbose(false);
            let guard = prepare(&mut checker, |_| {}, true);
            assert_eq!(guard.green, "\x1b[32m");
            assert!(!checker.generate.items);
            assert!(!checker.shrink.items);
            assert!(!checker.shrink.errors);
        }
    }
}
