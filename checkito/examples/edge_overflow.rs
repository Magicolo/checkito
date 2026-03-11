//! # Edge Case: Stack Overflow and Memory Exhaustion
//!
//! This example demonstrates scenarios that can cause stack overflow or extreme
//! memory usage using only the public API.
//!
//! Run with: cargo run --example edge_overflow -- <exploit_name>
//!
//! **WARNING**: Some of these tests intentionally consume large amounts of
//! memory or stack space. Run them individually and carefully.
//!
//! ## Findings
//!
//! 1. `collect_with(usize::MAX)` causes the library to try allocating a Vec
//!    of `usize::MAX` elements, leading to memory exhaustion or a capacity
//!    overflow panic. There is no upper bound on the count parameter.
//!
//! 2. Recursive generators using `dampen_with(0.0, usize::MAX, usize::MAX)`
//!    effectively disable dampening (see edge_nan.rs), and combined with
//!    `size(|_| 1.0)` to prevent natural size reduction, can produce very deep
//!    recursion. Depending on stack size, this may cause a stack overflow.
//!
//! 3. Regex patterns with huge repetition counts (e.g. `a{1000000}`) generate
//!    strings of that length, consuming proportional memory.
//!
//! 4. Deeply nested `flatten` chains each clone the State, accumulating memory.

use checkito::*;

fn collect_capacity_overflow() {
    println!("  [collect_capacity_overflow] collect_with(usize::MAX) at size=0 panics on allocation.");
    let result = std::panic::catch_unwind(|| {
        // Even at size=0, the usize range (usize::MAX..=usize::MAX) resolves
        // to usize::MAX elements, triggering a capacity overflow.
        let gen = (0u8..=1).collect_with::<_, Vec<u8>>(usize::MAX);
        let _v = gen.sample(0.0);
    });
    match &result {
        Ok(_) => println!("    -> Unexpectedly succeeded (system has enormous RAM?)."),
        Err(e) => {
            let msg = e
                .downcast_ref::<String>()
                .map(|s| s.as_str())
                .or_else(|| e.downcast_ref::<&str>().copied())
                .unwrap_or("(non-string panic)");
            println!("    -> Confirmed: panicked with: {msg}");
        }
    }
    assert!(
        result.is_err(),
        "Expected capacity overflow panic from collect_with(usize::MAX)"
    );
}

fn collect_huge_at_full_size() {
    println!("  [collect_huge_at_full_size] collect_with(0..=10_000_000) at size=1.0 allocates ~10M elements.");
    // This is not a panic, but it demonstrates that large count ranges can
    // cause significant memory allocation. We limit to 10M to avoid OOM.
    let gen = (0u8..=1).collect_with::<_, Vec<u8>>(0usize..=10_000_000);
    let v = gen.sample(1.0);
    println!("    -> Allocated Vec of {} elements ({} bytes).", v.len(), v.len());
    assert!(v.len() <= 10_000_001);
}

fn regex_huge_repetition() {
    println!("  [regex_huge_repetition] regex with large repeat count generates huge strings.");
    // regex("a{{100000}}", None) would try to generate a 100K character string.
    // We use a smaller count to avoid OOM but demonstrate the issue.
    let gen = regex("a{10000}", None).expect("valid regex");
    let s = gen.sample(1.0);
    println!("    -> Generated string of length {} from 'a{{10000}}'.", s.len());
    assert_eq!(s.len(), 10000, "Expected exactly 10000 'a' characters");
}

fn deeply_nested_flatten() {
    println!("  [deeply_nested_flatten] Deeply nested flatten chains accumulate state clones.");

    // Each flatten level clones the State. 100 levels is manageable but
    // demonstrates the pattern. In theory, 10000+ levels could cause issues.
    let base = same(42u32);
    // Build a chain: same(same(same(...same(42)...)))
    // We can't do this dynamically in the type system, so we use boxed.
    let gen: boxed::Boxed<u32> = same(42u32).boxed();
    // Wrap in 50 layers of flatten via flat_map(identity).
    let mut gen = gen;
    for _ in 0..50 {
        gen = gen.flat_map(|v| same(v)).boxed();
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| gen.sample(0.5)));
    match result {
        Ok(v) => println!("    -> 50-deep flatten produced: {v} (no crash)."),
        Err(_) => println!("    -> Panicked (stack overflow or other issue)."),
    }
    let _ = base; // prevent unused warning
}

fn checker_generate_zero_count() {
    println!("  [checker_generate_zero_count] Checker with generate.count=0 produces no results.");
    let mut checker = (0u8..=255).checker();
    checker.generate.count = 0;

    let mut count = 0;
    for _result in checker.checks(|_x: u8| true) {
        count += 1;
    }
    println!("    -> {count} results with generate.count=0.");
    assert_eq!(count, 0, "Expected no results with count=0");
}

fn checker_shrink_zero_count() {
    println!("  [checker_shrink_zero_count] Checker with shrink.count=0 skips shrinking entirely.");
    let mut checker = (0u32..1_000_000).checker();
    checker.generate.count = 100;
    checker.shrink.count = 0;

    let fail = checker.check(|x| x < 500);
    match fail {
        Some(fail) => {
            println!(
                "    -> Failure found: item={}, shrinks={}.",
                fail.item, fail.shrinks
            );
            // With shrink.count=0, the item should NOT be shrunk to the minimal
            // value. It should be whatever was first generated.
            println!(
                "    -> Item is {} (unshrunk, likely > 500).",
                fail.item
            );
        }
        None => println!("    -> No failure found (unlikely with 100 cases over 0..1M)."),
    }
}

fn main() {
    let exploits: &[(&str, fn())] = &[
        ("collect_capacity_overflow", collect_capacity_overflow),
        ("collect_huge_at_full_size", collect_huge_at_full_size),
        ("regex_huge_repetition", regex_huge_repetition),
        ("deeply_nested_flatten", deeply_nested_flatten),
        ("checker_generate_zero_count", checker_generate_zero_count),
        ("checker_shrink_zero_count", checker_shrink_zero_count),
    ];

    let args: Vec<String> = std::env::args().collect();
    let filter = args.get(1).map(|s| s.as_str());

    println!("=== Edge Case: Overflow and Resource Exhaustion ===\n");

    let mut ran = 0;
    for (name, exploit) in exploits {
        if filter.is_some_and(|f| f != *name) {
            continue;
        }
        ran += 1;
        exploit();
        println!();
    }

    println!("--- {ran} exploits executed ---");
}
