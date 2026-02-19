#![cfg(feature = "asynchronous")]

pub mod common;
use common::*;
use core::{
    future::{Future, ready},
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering},
};
use futures_lite::{
    StreamExt,
    future::{block_on, yield_now},
};

#[test]
fn executes_to_completion() {
    let fail = block_on(
        usize::generator()
            .checker()
            .asynchronous(None)
            .check(|value| async move { value < 1_000_000 }),
    );
    assert!(fail.is_some());
}

#[test]
fn handles_async_panics_gracefully() {
    let result_outer = block_on(
        (1u8..=255)
            .checker()
            .asynchronous(None)
            .check(|_value| {
                panic!();
                #[allow(unreachable_code)]
                ready(true)
            }),
    );
    let result_inner = block_on(
        (1u8..=255)
            .checker()
            .asynchronous(None)
            .check(|_value| async move {
                panic!();
                #[allow(unreachable_code)]
                true
            }),
    );
    assert!(matches!(
        result_outer,
        Some(Fail {
            cause: Cause::Panic(..),
            ..
        })
    ));
    assert!(matches!(
        result_inner,
        Some(Fail {
            cause: Cause::Panic(..),
            ..
        })
    ));
}

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check]
    async fn compiles_with_async_function() {}

    #[check(asynchronous = true)]
    fn compiles_with_asynchronous_option() -> impl Future<Output = ()> {
        ready(())
    }

    #[check]
    async fn async_check_runs_correctly() -> bool {
        futures_lite::future::yield_now().await;
        true
    }

    #[check(_, generate.count = 10)]
    async fn async_check_with_parameter(_value: u16) {
        futures_lite::future::yield_now().await;
        // Just test that async check with parameter compiles and runs
    }

    #[check]
    async fn async_check_returns_unit() {
        futures_lite::future::yield_now().await;
    }

    #[check(..)]
    async fn synchronous_and_asynchronous_produce_same_results(
        seed: u64,
        maximum: u8,
        generates: (usize, bool),
        shrinks: (usize, bool, bool),
        exhaustive: Option<bool>,
    ) {
        // Collect sync results
        let mut checker = u8::generator().checker();
        checker.generate.seed = seed;
        checker.generate.count = generates.0;
        checker.generate.items = generates.1;
        checker.generate.exhaustive = exhaustive;
        checker.shrink.count = shrinks.0;
        checker.shrink.items = shrinks.1;
        checker.shrink.errors = shrinks.2;
        let synchronous = checker.checks(|value| value < maximum).collect::<Vec<_>>();

        // Collect async results with concurrency=1 for deterministic behavior
        let mut checker = u8::generator().checker();
        checker.generate.seed = seed;
        checker.generate.count = generates.0;
        checker.generate.items = generates.1;
        checker.generate.exhaustive = exhaustive;
        checker.shrink.count = shrinks.0;
        checker.shrink.items = shrinks.1;
        checker.shrink.errors = shrinks.2;
        let asynchronous = checker
            .asynchronous(NonZeroUsize::new(1))
            .checks(|value| async move { value < maximum })
            .collect::<Vec<_>>()
            .await;

        assert_eq!(synchronous, asynchronous);
    }

    #[check(1..32usize, ..)]
    async fn respects_concurrency_parameter(concurrency: usize, wait: u8) {
        let counter = AtomicUsize::new(0);
        let concurrent = AtomicUsize::new(0);
        i32::generator()
            .checker()
            .asynchronous(NonZeroUsize::new(concurrency))
            .check(|_| async {
                let current = counter.fetch_add(1, Ordering::SeqCst) + 1;
                concurrent.fetch_max(current, Ordering::SeqCst);

                for _ in 0..wait {
                    yield_now().await;
                }

                counter.fetch_sub(1, Ordering::SeqCst);
                true
            })
            .await;

        let concurrent = concurrent.into_inner();
        assert!(concurrent <= concurrency);
    }
}
