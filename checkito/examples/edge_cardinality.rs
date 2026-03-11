//! # Edge Case: Cardinality Override Exploits
//!
//! This example demonstrates how `cardinality::<_, N>()` can be used to deceive
//! the checking engine into incorrect behavior. The cardinality override is a
//! power tool that, when misused, leads to surprising results.
//!
//! Run with: cargo run --example edge_cardinality
//!
//! ## Findings
//!
//! 1. Setting cardinality to 0 with `cardinality::<_, 0>()` makes the checker
//!    believe there are zero possible values. In exhaustive mode, this results
//!    in zero test iterations — the check succeeds vacuously even for
//!    properties that should fail.
//!
//! 2. Setting cardinality to 1 makes the checker think there's only one value,
//!    so it runs exactly once in exhaustive mode. A property that fails on some
//!    values but not the first will pass incorrectly.
//!
//! 3. Forcing exhaustive mode on a generator with `None` cardinality via
//!    `generate.exhaustive = Some(true)` causes 0 iterations because
//!    `Modes::Exhaustive(count)` where count comes from the cardinality
//!    (which is 0 by fallback).
//!
//! 4. A generator whose dynamic cardinality disagrees with its static
//!    cardinality can confuse exhaustive checking. The dynamic cardinality
//!    is used for exhaustive mode decisions, while static is used for
//!    compile-time optimizations.

use checkito::*;

fn cardinality_zero_vacuous_pass() {
    println!("  [cardinality_zero_vacuous_pass] cardinality::<_, 0>() makes check pass vacuously.");

    // This generator claims it has 0 possible values.
    let gen = cardinality::<_, 0>(0u32..1000);

    // The checker sees cardinality=0, which is <= default count (1024),
    // so it enters exhaustive mode with 0 iterations.
    let fail = gen.check(|x| {
        // This property ALWAYS fails, but the check never runs!
        assert!(x > 999_999, "This should always fail");
    });

    match fail {
        None => println!("    -> Check passed vacuously! No iterations were performed."),
        Some(f) => println!("    -> Unexpectedly found failure: {:?}", f.item),
    }

    // Verify: the checks iterator produces 0 items.
    let count = gen.checker().checks(|_: u32| false).count();
    println!("    -> Total check iterations: {count}");
    assert_eq!(count, 0, "Expected 0 iterations with cardinality 0");
}

fn cardinality_one_single_check() {
    println!("  [cardinality_one_single_check] cardinality::<_, 1>() runs only one test case.");

    let gen = cardinality::<_, 1>(0u32..1000);

    // With cardinality=1, the checker runs exactly once (exhaustive with index=0).
    // The first generated value is 0 (exhaustive starts from 0).
    let fail = gen.check(|x| {
        // Fails for x >= 500, but exhaustive only tests x=0, which passes.
        x < 500
    });

    match fail {
        None => println!("    -> Check passed! Only tested first value (0), missed failures at 500+."),
        Some(f) => println!("    -> Found failure: {:?}", f.item),
    }

    let count = gen.checker().checks(|_: u32| true).count();
    println!("    -> Total check iterations: {count}");
    assert_eq!(count, 1, "Expected exactly 1 iteration with cardinality 1");
}

fn cardinality_lie_small() {
    println!("  [cardinality_lie_small] Lying about cardinality (claiming 5 for 0..1000) limits exhaustive checks.");

    let gen = cardinality::<_, 5>(0u32..1000);

    // The checker runs 5 exhaustive iterations for a generator with 1000 values.
    let mut values = Vec::new();
    for result in gen.checker().checks(|_x: u32| {
        // Capture the value.
        true
    }) {
        values.push(*result);
    }
    println!("    -> Exhaustive check ran {} iterations (should be 5).", values.len());
    assert_eq!(values.len(), 5, "Expected 5 iterations");
}

fn forced_exhaustive_unknown_cardinality() {
    println!("  [forced_exhaustive_unknown_cardinality] Forcing exhaustive on boxed generator.");

    // A boxed generator always reports CARDINALITY=None (dynamic may differ).
    let gen = (0u32..100).boxed();

    let mut checker = gen.checker();
    checker.generate.exhaustive = Some(true);
    checker.generate.count = 50;

    // With exhaustive forced and count=50, it should run up to 50 exhaustive
    // iterations (the Modes::Exhaustive(count) uses generate.count).
    let mut count = 0;
    for _result in checker.checks(|_: u32| true) {
        count += 1;
    }
    println!("    -> Forced exhaustive produced {count} iterations.");
    // Exhaustive mode with count=50 should give 50 iterations.
}

fn cardinality_overflow_static() {
    println!("  [cardinality_overflow_static] cardinality::<_, {{u128::MAX}}>() prevents exhaustive mode.");

    let gen = cardinality::<_, { u128::MAX }>(0u8..=1);

    // u128::MAX is definitely > default count (1024), so random mode is used.
    let mut checker = gen.checker();
    checker.generate.count = 10;

    let mut count = 0;
    for _result in checker.checks(|_: u8| true) {
        count += 1;
    }
    println!("    -> With cardinality=u128::MAX and count=10: {count} iterations (random mode).");
    assert_eq!(count, 10, "Expected 10 random iterations");
}

fn cardinality_mismatch_static_dynamic() {
    println!("  [cardinality_mismatch_static_dynamic] Static vs dynamic cardinality can disagree.");

    // The `cardinality::<_, N>()` override only sets the static CARDINALITY const.
    // But the `cardinality()` method also returns `Some(N)` since the impl overrides both.
    // Actually, looking at the code: `Cardinality<G, C>` sets CARDINALITY = Some(C)
    // and does NOT override `cardinality()`, so it inherits G's dynamic cardinality.
    //
    // Wait, let me check... The Cardinality struct doesn't implement `cardinality()`,
    // so it uses the default which returns `Self::CARDINALITY` = `Some(C)`.
    // Actually the default impl is `fn cardinality(&self) -> Option<u128> { Self::CARDINALITY }`.
    // So both static and dynamic are overridden to `Some(C)`.

    let gen = cardinality::<_, 42>(0u8..=255);
    println!("    -> Static cardinality: 42 (overridden from 256)");
    println!("    -> This means exhaustive checks only run 42 iterations for a 256-value space.");

    let mut count = 0;
    for _result in gen.checker().checks(|_: u8| true) {
        count += 1;
    }
    println!("    -> Ran {count} iterations (expected 42 in exhaustive mode).");
}

fn main() {
    println!("=== Edge Case: Cardinality Override Exploits ===\n");

    cardinality_zero_vacuous_pass();
    println!();
    cardinality_one_single_check();
    println!();
    cardinality_lie_small();
    println!();
    forced_exhaustive_unknown_cardinality();
    println!();
    cardinality_overflow_static();
    println!();
    cardinality_mismatch_static_dynamic();

    println!("\n--- All cardinality exploits completed ---");
}
