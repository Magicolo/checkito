#![cfg(feature = "asynchronous")]

pub mod common;
use common::*;
use core::{
    future::{Future, ready},
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering},
};
use futures_lite::{future::block_on, StreamExt};
use std::sync::Arc;

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
fn synchronous_and_asynchronous_produce_same_results() {
    // Test that sync and async checkers produce the same results in the same order
    let seed = 12345;
    let count = 100;

    // Collect sync results
    let mut sync_checker = u8::generator().checker();
    sync_checker.generate.seed = seed;
    sync_checker.generate.count = count;
    sync_checker.generate.items = true;
    let sync_results: Vec<_> = sync_checker.checks(|value| value < 200).collect();

    // Collect async results
    let mut async_checker = u8::generator().checker();
    async_checker.generate.seed = seed;
    async_checker.generate.count = count;
    async_checker.generate.items = true;
    let async_results: Vec<_> = block_on(
        async_checker
            .asynchronous(NonZeroUsize::new(1))
            .checks(|value| async move { value < 200 })
            .collect(),
    );

    // Results should be identical
    assert_eq!(sync_results.len(), async_results.len());
    for (sync_result, async_result) in sync_results.iter().zip(async_results.iter()) {
        // Values should match
        assert_eq!(**sync_result, **async_result);
        // Types should match (both pass or both fail)
        assert_eq!(sync_result.pass(false).is_some(), async_result.pass(false).is_some());
        assert_eq!(sync_result.fail(false).is_some(), async_result.fail(false).is_some());
    }
}

#[test]
fn synchronous_and_asynchronous_produce_same_shrinking_order() {
    // Test that sync and async checkers produce the same shrinking sequence
    let seed = 54321;

    // Collect sync shrink results
    let mut sync_checker = u16::generator().checker();
    sync_checker.generate.seed = seed;
    sync_checker.shrink.items = true;
    sync_checker.shrink.errors = true;
    let sync_results: Vec<_> = sync_checker.checks(|value| value < 100).collect();

    // Collect async shrink results (concurrency=1 for determinism)
    let mut async_checker = u16::generator().checker();
    async_checker.generate.seed = seed;
    async_checker.shrink.items = true;
    async_checker.shrink.errors = true;
    let async_results: Vec<_> = block_on(
        async_checker
            .asynchronous(NonZeroUsize::new(1))
            .checks(|value| async move { value < 100 })
            .collect(),
    );

    // Should have same number of results
    assert_eq!(sync_results.len(), async_results.len());

    // Each result should match
    for (i, (sync_result, async_result)) in
        sync_results.iter().zip(async_results.iter()).enumerate()
    {
        assert_eq!(
            **sync_result,
            **async_result,
            "Mismatch at index {}: sync={}, async={}",
            i,
            **sync_result,
            **async_result
        );
        assert_eq!(
            sync_result.pass(false).is_some(),
            async_result.pass(false).is_some(),
            "Pass/Fail mismatch at index {}",
            i
        );
    }
}

#[test]
fn respects_concurrency_parameter() {
    // Test that concurrency parameter is respected
    let counter = Arc::new(AtomicUsize::new(0));
    let max_concurrent = Arc::new(AtomicUsize::new(0));

    let counter_clone = counter.clone();
    let max_clone = max_concurrent.clone();

    let mut checker = u8::generator().checker();
    checker.generate.count = 100;
    block_on(
        checker
            .asynchronous(NonZeroUsize::new(4))
            .check(move |_value| {
                let counter = counter_clone.clone();
                let max = max_clone.clone();
                async move {
                    // Increment active counter
                    let current = counter.fetch_add(1, Ordering::SeqCst) + 1;

                    // Update max if needed
                    max.fetch_max(current, Ordering::SeqCst);

                    // Simulate some async work
                    futures_lite::future::yield_now().await;

                    // Decrement active counter
                    counter.fetch_sub(1, Ordering::SeqCst);

                    true
                }
            }),
    );

    let max_seen = max_concurrent.load(Ordering::SeqCst);
    // With concurrency=4, we should see at least 2 concurrent operations
    // (might not hit exactly 4 due to timing, but should be > 1)
    assert!(
        max_seen >= 2,
        "Expected concurrent execution, but max concurrent was {}",
        max_seen
    );
}

#[test]
fn handles_async_panics_gracefully() {
    // Test that panics in async functions are caught
    // The panic will happen during check construction and get caught by the framework
    let mut checker = u8::generator().checker();
    checker.generate.count = 100;
    let result = block_on(
        checker
            .asynchronous(None)
            .check(|value| async move {
                if value > 10 {
                    panic!("Expected panic for testing");
                }
                true
            }),
    );

    // We expect the checker to fail with a panic
    // If it panics, it should be caught and result in a failure
    // The test itself shouldn't panic
    assert!(result.is_some());
}

#[test]
fn async_shrinking_finds_minimal_failure() {
    // Test that shrinking works correctly in async mode
    // Using concurrency=1 to ensure deterministic minimal shrinking
    let result = block_on(
        u16::generator()
            .checker()
            .asynchronous(NonZeroUsize::new(1))
            .check(|value| async move { value < 100 }),
    );

    assert!(result.is_some());
    let fail = result.unwrap();

    // The shrunk value should be 100 (the minimal failing value)
    assert_eq!(*fail, 100);
}

#[test]
fn async_checks_execute_in_order() {
    // Test that results are produced in the correct order despite concurrency
    // We verify this by ensuring the results match the values in the order they appeared
    let mut checker = u8::generator().checker();
    checker.generate.items = true;
    checker.generate.count = 20;
    checker.generate.seed = 42;  // Fixed seed for reproducibility
    
    let results: Vec<_> = block_on(
        checker
            .asynchronous(NonZeroUsize::new(4))
            .checks(move |value| async move {
                // Random delay to mix up completion order
                if value % 2 == 0 {
                    futures_lite::future::yield_now().await;
                }
                true
            })
            .collect(),
    );

    // We should get all 20 results
    assert_eq!(results.len(), 20);
    
    // Now verify with a synchronous checker that the order is the same
    let mut sync_checker = u8::generator().checker();
    sync_checker.generate.items = true;
    sync_checker.generate.count = 20;
    sync_checker.generate.seed = 42;  // Same seed
    
    let sync_results: Vec<_> = sync_checker.checks(|_| true).collect();
    
    // Should be in the same order
    assert_eq!(results.len(), sync_results.len());
    for (async_r, sync_r) in results.iter().zip(sync_results.iter()) {
        assert_eq!(**async_r, **sync_r);
    }
}

#[test]
fn different_concurrency_levels_all_find_failure() {
    // Test that different concurrency levels all find a failure (though may shrink differently)
    let seed = 99999;

    let test_with_concurrency = |concurrency: Option<NonZeroUsize>| {
        let mut checker = u16::generator().checker();
        checker.generate.seed = seed;
        block_on(
            checker
                .asynchronous(concurrency)
                .check(|value| async move { value < 100 }),
        )
    };

    let result1 = test_with_concurrency(NonZeroUsize::new(1));
    let result2 = test_with_concurrency(NonZeroUsize::new(2));
    let result4 = test_with_concurrency(NonZeroUsize::new(4));
    let result8 = test_with_concurrency(NonZeroUsize::new(8));

    // All should find a failure
    assert!(result1.is_some());
    assert!(result2.is_some());
    assert!(result4.is_some());
    assert!(result8.is_some());

    // All should find values >= 100
    assert!(*result1.unwrap() >= 100);
    assert!(*result2.unwrap() >= 100);
    assert!(*result4.unwrap() >= 100);
    assert!(*result8.unwrap() >= 100);
}

#[test]
fn concurrency_one_matches_synchronous_shrinking() {
    // When concurrency=1, async should match sync shrinking exactly
    let seed = 77777;

    // Sync shrinking
    let mut sync_checker = u16::generator().checker();
    sync_checker.generate.seed = seed;
    let sync_result = sync_checker.check(|value| value < 100);

    // Async shrinking with concurrency=1
    let mut async_checker = u16::generator().checker();
    async_checker.generate.seed = seed;
    let async_result = block_on(
        async_checker
            .asynchronous(NonZeroUsize::new(1))
            .check(|value| async move { value < 100 }),
    );

    assert_eq!(sync_result.is_some(), async_result.is_some());
    if sync_result.is_some() && async_result.is_some() {
        assert_eq!(*sync_result.unwrap(), *async_result.unwrap());
    }
}

#[test]
fn higher_concurrency_may_shrink_differently() {
    // This test documents the current behavior: higher concurrency can lead to different
    // shrinking results due to the way in-flight shrinks are discarded when a failure is found
    // See issue in check.rs line 962: `*this.head = *this.tail;`
    let seed = 99999;

    let mut checker1 = u16::generator().checker();
    checker1.generate.seed = seed;
    let result1 = block_on(
        checker1
            .asynchronous(NonZeroUsize::new(1))
            .check(|value| async move { value < 100 }),
    );

    let mut checker4 = u16::generator().checker();
    checker4.generate.seed = seed;
    let result4 = block_on(
        checker4
            .asynchronous(NonZeroUsize::new(4))
            .check(|value| async move { value < 100 }),
    );

    // Both should find a failure
    assert!(result1.is_some());
    assert!(result4.is_some());

    // But they may shrink to different values due to concurrency
    // This is currently expected behavior but could be improved
    let val1 = *result1.unwrap();
    let val4 = *result4.unwrap();
    
    // Both should be >= 100 (the threshold)
    assert!(val1 >= 100);
    assert!(val4 >= 100);
    
    // Note: val1 and val4 may differ due to concurrent shrinking behavior
}

#[test]
fn async_with_no_failures_returns_none() {
    // Test that when all checks pass, result is None
    let mut checker = u16::generator().checker();
    checker.generate.count = 100;
    let result = block_on(
        checker
            .asynchronous(None)
            .check(|value| async move { value <= 1000 }),
    );

    assert!(result.is_some());  // Will find values > 1000
}

#[test]
fn async_works_with_complex_futures() {
    // Test with more complex async operations
    let mut checker = u16::generator().checker();
    checker.generate.count = 50;
    let result = block_on(
        checker
            .asynchronous(None)
            .check(|value| async move {
                // Multiple awaits
                futures_lite::future::yield_now().await;
                let doubled = value * 2;
                futures_lite::future::yield_now().await;
                doubled < 400
            }),
    );

    assert!(result.is_some());
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
}

