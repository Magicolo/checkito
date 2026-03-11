//! # Edge Case: Empty and Degenerate Generators
//!
//! This example explores the behavior of the library when generators produce
//! no values, or when combinators are applied to degenerate inputs.
//!
//! Run with: cargo run --example edge_empty
//!
//! ## Findings
//!
//! 1. `Any` over an empty slice (`[].as_slice().any()`) always produces `None`
//!    from its shrinker, since `any_uniform` on an empty iterator returns
//!    `None`. This is correct but may surprise users who expect a compile-time
//!    or runtime error.
//!
//! 2. A filter that always rejects does NOT cause an infinite loop because
//!    `Filter::generate` does not retry — it generates once and the shrinker
//!    yields `None` when the predicate fails. The `Option<T>` wrapper handles
//!    this cleanly.
//!
//! 3. `collect_with(0)` produces empty collections, which is correct behavior.
//!
//! 4. `array::<0>()` produces empty arrays `[T; 0]`, which is correct.
//!
//! 5. `samples(0)` produces an empty iterator, which is correct.
//!
//! 6. The `Same` generator wrapping a non-Clone type that's been cloned many
//!    times still works fine (Clone is required by the trait bound).

use checkito::*;

fn any_empty_slice() {
    println!("  [any_empty_slice] Any over empty slice always produces None.");

    let generators: Vec<std::ops::RangeInclusive<u8>> = vec![];
    let gen = generators.any();

    let mut none_count = 0;
    let total = 100;
    for sample in gen.samples(total) {
        if sample.is_none() {
            none_count += 1;
        }
    }
    println!("    -> {none_count}/{total} samples were None.");
    assert_eq!(none_count, total, "All samples from empty any() should be None");
}

fn filter_always_reject() {
    println!("  [filter_always_reject] Filter that always rejects produces Option::None, no infinite loop.");

    let gen = Generate::filter(0u32..1000, |_| false);

    let mut none_count = 0;
    let total = 100;
    for sample in gen.samples(total) {
        let sample: Option<u32> = sample;
        if sample.is_none() {
            none_count += 1;
        }
    }
    println!("    -> {none_count}/{total} samples were None (all rejected).");
    assert_eq!(none_count, total, "All samples should be None when filter always rejects");
}

fn filter_map_always_reject() {
    println!("  [filter_map_always_reject] FilterMap that always returns None, no infinite loop.");

    let gen = Generate::filter_map(0u32..1000, |_| None::<u32>);

    let mut none_count = 0;
    let total = 100;
    for sample in gen.samples(total) {
        let sample: Option<u32> = sample;
        if sample.is_none() {
            none_count += 1;
        }
    }
    println!("    -> {none_count}/{total} samples were None.");
    assert_eq!(none_count, total);
}

fn collect_zero_count() {
    println!("  [collect_zero_count] collect_with(0) produces empty collections.");

    let gen = (0u8..=255).collect_with::<_, Vec<u8>>(0usize);
    let v = gen.sample(1.0);
    println!("    -> collect_with(0) produced Vec of length {}.", v.len());
    assert_eq!(v.len(), 0);
}

fn collect_zero_range() {
    println!("  [collect_zero_range] collect_with(0..=0) produces empty collections.");

    let gen = (0u8..=255).collect_with::<_, Vec<u8>>(0usize..=0);
    let v = gen.sample(1.0);
    println!("    -> collect_with(0..=0) produced Vec of length {}.", v.len());
    assert_eq!(v.len(), 0);
}

fn array_zero() {
    println!("  [array_zero] array::<0>() produces empty arrays.");

    let gen = (0u8..=255).array::<0>();
    let arr = gen.sample(0.5);
    println!("    -> array::<0>() produced array of length {}.", arr.len());
    assert_eq!(arr.len(), 0);
}

fn samples_zero() {
    println!("  [samples_zero] samples(0) produces empty iterator.");

    let count = (0u8..=255).samples(0).count();
    println!("    -> samples(0) produced {count} items.");
    assert_eq!(count, 0);
}

fn check_always_pass() {
    println!("  [check_always_pass] Check with property that always passes returns None.");

    let fail = (0u32..1000).check(|_| true);
    match &fail {
        None => println!("    -> Correctly returned None (no failure)."),
        Some(f) => println!("    -> Unexpected failure: {}", f.item),
    }
    assert!(fail.is_none());
}

fn check_always_fail() {
    println!("  [check_always_fail] Check with property that always fails returns a Fail.");

    let fail = (0u32..1000).check(|_| false);
    match &fail {
        Some(f) => println!("    -> Found failure at item={}, shrinks={}.", f.item, f.shrinks),
        None => println!("    -> Unexpected pass!"),
    }
    assert!(fail.is_some());
    // The shrunk value should be 0 (the minimal value for the range).
    assert_eq!(fail.unwrap().item, 0, "Minimal failing value should be 0");
}

fn same_generator_determinism() {
    println!("  [same_generator_determinism] same(42) always produces 42.");

    let gen = same(42u32);
    let mut all_42 = true;
    for sample in gen.samples(100) {
        if sample != 42 {
            all_42 = false;
            break;
        }
    }
    println!("    -> All 100 samples were 42: {all_42}.");
    assert!(all_42);
}

fn same_generator_cardinality() {
    println!("  [same_generator_cardinality] same(42) has cardinality 1, runs exhaustively once.");

    let gen = same(42u32);
    let count = gen.checker().checks(|x: u32| x == 42).count();
    println!("    -> Exhaustive check ran {count} iteration(s).");
    assert_eq!(count, 1, "same() should run exactly once in exhaustive mode");
}

fn filter_during_shrink() {
    println!("  [filter_during_shrink] Shrinking a filtered value can lose the filter constraint.");

    // A filter produces Option<T>. During shrinking, the inner value may shrink
    // to something that no longer passes the filter, in which case the shrinker
    // yields None. This is documented behavior but may surprise users.
    let gen = Generate::filter(0u32..1000, |&x| x % 2 == 0);

    let fail = gen.check(|x: Option<u32>| {
        // Pass if None or if < 100.
        x.map_or(true, |v| v < 100)
    });

    match &fail {
        Some(f) => {
            println!(
                "    -> Failure shrunk to: {:?} (shrinks: {}).",
                f.item, f.shrinks
            );
            // The shrunk value should be Some(100) since it's the smallest even
            // number >= 100.
            if let Some(v) = f.item {
                println!("    -> Shrunk value {v} is even: {}", v % 2 == 0);
            }
        }
        None => println!("    -> No failure found (possible if filter always rejects)."),
    }
}

fn main() {
    println!("=== Edge Case: Empty and Degenerate Generators ===\n");

    any_empty_slice();
    println!();
    filter_always_reject();
    println!();
    filter_map_always_reject();
    println!();
    collect_zero_count();
    println!();
    collect_zero_range();
    println!();
    array_zero();
    println!();
    samples_zero();
    println!();
    check_always_pass();
    println!();
    check_always_fail();
    println!();
    same_generator_determinism();
    println!();
    same_generator_cardinality();
    println!();
    filter_during_shrink();

    println!("\n--- All empty/degenerate generator tests completed ---");
}
