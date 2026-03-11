//! # Edge Case: Assertion Panics from Public API
//!
//! This example demonstrates several ways the public API can trigger assertion
//! panics (not graceful errors) through seemingly innocent usage. Each function
//! is a self-contained exploit that panics via `assert!` inside the library.
//!
//! Run individual exploits with:
//!   cargo run --example edge_panic -- <exploit_name>
//!
//! ## Findings
//!
//! 1. `Weight::new` panics on NaN, infinity, zero, and negative weights with
//!    bare `assert!` (no helpful error message). A user accidentally computing
//!    a weight from division (e.g. `0.0 / 0.0`) will see an opaque panic.
//!
//! 2. `Sample::sample` panics when given `f64::NAN`, `f64::INFINITY`, or
//!    `f64::NEG_INFINITY` as a size because `Sizes::new` asserts finiteness
//!    before clamping. A user passing an unvalidated size from external input
//!    (e.g. a configuration file) will crash with no explanation.
//!
//! 3. The `size` combinator panics if the closure returns a NaN or infinite
//!    value, again hitting the `Sizes::new` assertion.
//!
//! 4. Float range generation asserts `start.is_finite() && end.is_finite()`,
//!    meaning any range containing NaN or infinity panics instead of producing
//!    clamped or wrapped behavior.

use checkito::*;
use std::process;

fn weight_nan() {
    println!("  [weight_nan] Weight::new(f64::NAN, ...) panics with opaque assert!");
    let result = std::panic::catch_unwind(|| {
        // A user might compute a weight from data: 0.0 / 0.0 = NaN.
        let _w = state::Weight::new(f64::NAN, 0u8..=255);
    });
    assert!(result.is_err(), "Expected panic from Weight::new(NaN)");
    println!("    -> Confirmed: panicked with assertion error (no helpful message).");
}

fn weight_infinity() {
    println!("  [weight_infinity] Weight::new(f64::INFINITY, ...) panics!");
    let result = std::panic::catch_unwind(|| {
        let _w = state::Weight::new(f64::INFINITY, 0u8..=255);
    });
    assert!(result.is_err(), "Expected panic from Weight::new(INFINITY)");
    println!("    -> Confirmed: panicked.");
}

fn weight_neg_infinity() {
    println!("  [weight_neg_infinity] Weight::new(f64::NEG_INFINITY, ...) panics!");
    let result = std::panic::catch_unwind(|| {
        let _w = state::Weight::new(f64::NEG_INFINITY, 0u8..=255);
    });
    assert!(result.is_err(), "Expected panic from Weight::new(NEG_INFINITY)");
    println!("    -> Confirmed: panicked.");
}

fn weight_zero() {
    println!("  [weight_zero] Weight::new(0.0, ...) panics!");
    let result = std::panic::catch_unwind(|| {
        let _w = state::Weight::new(0.0, 0u8..=255);
    });
    assert!(result.is_err(), "Expected panic from Weight::new(0.0)");
    println!("    -> Confirmed: panicked.");
}

fn weight_negative() {
    println!("  [weight_negative] Weight::new(-1.0, ...) panics!");
    let result = std::panic::catch_unwind(|| {
        let _w = state::Weight::new(-1.0, 0u8..=255);
    });
    assert!(result.is_err(), "Expected panic from Weight::new(-1.0)");
    println!("    -> Confirmed: panicked.");
}

fn weight_tiny() {
    println!("  [weight_tiny] Weight::new(f64::EPSILON / 2.0, ...) panics!");
    let result = std::panic::catch_unwind(|| {
        // A very small but positive weight.
        let _w = state::Weight::new(f64::EPSILON / 2.0, 0u8..=255);
    });
    assert!(
        result.is_err(),
        "Expected panic from Weight::new(EPSILON/2)"
    );
    println!("    -> Confirmed: panicked (value is positive but below EPSILON threshold).");
}

fn sample_nan() {
    println!("  [sample_nan] (0u8..=255).sample(f64::NAN) panics!");
    let result = std::panic::catch_unwind(|| {
        let _v = (0u8..=255).sample(f64::NAN);
    });
    assert!(result.is_err(), "Expected panic from sample(NaN)");
    println!("    -> Confirmed: panicked (Sizes::new asserts finiteness).");
}

fn sample_infinity() {
    println!("  [sample_infinity] (0u8..=255).sample(f64::INFINITY) panics!");
    let result = std::panic::catch_unwind(|| {
        let _v = (0u8..=255).sample(f64::INFINITY);
    });
    assert!(result.is_err(), "Expected panic from sample(INFINITY)");
    println!("    -> Confirmed: panicked.");
}

fn sample_neg_infinity() {
    println!("  [sample_neg_infinity] (0u8..=255).sample(f64::NEG_INFINITY) panics!");
    let result = std::panic::catch_unwind(|| {
        let _v = (0u8..=255).sample(f64::NEG_INFINITY);
    });
    assert!(
        result.is_err(),
        "Expected panic from sample(NEG_INFINITY)"
    );
    println!("    -> Confirmed: panicked.");
}

fn size_combinator_nan() {
    println!("  [size_combinator_nan] gen.size(|_| f64::NAN).sample(0.5) panics!");
    let result = std::panic::catch_unwind(|| {
        // The size combinator converts the closure result to Sizes via Into.
        // f64::NAN -> Range(NaN, NaN) -> Sizes::new(NaN, NaN, SCALE) -> assert! panic.
        let gen = (0u8..=255).size(|_| f64::NAN);
        let _v = gen.sample(0.5);
    });
    assert!(result.is_err(), "Expected panic from size(NaN)");
    println!("    -> Confirmed: panicked.");
}

fn size_combinator_infinity() {
    println!("  [size_combinator_infinity] gen.size(|_| f64::INFINITY).sample(0.5) panics!");
    let result = std::panic::catch_unwind(|| {
        let gen = (0u8..=255).size(|_| f64::INFINITY);
        let _v = gen.sample(0.5);
    });
    assert!(result.is_err(), "Expected panic from size(INFINITY)");
    println!("    -> Confirmed: panicked.");
}

fn float_range_nan() {
    println!("  [float_range_nan] Generating from a float range with NaN endpoint panics!");
    let result = std::panic::catch_unwind(|| {
        // Float generation has an explicit `assert!(start.is_finite() && end.is_finite())`.
        // Constructing a Range<f64> directly from NaN values triggers this.
        let _v = (f64::NAN..=1.0).sample(0.5);
    });
    assert!(result.is_err(), "Expected panic from NaN float range");
    println!("    -> Confirmed: panicked.");
}

fn float_range_infinity() {
    println!("  [float_range_infinity] Generating from f64::INFINITY..=f64::INFINITY panics!");
    let result = std::panic::catch_unwind(|| {
        // The range conversion for floats normalizes INF..=INF to Range(INF, INF).
        // Then float generate() asserts start.is_finite(), causing a panic.
        let _v = (f64::INFINITY..=f64::INFINITY).sample(0.5);
    });
    assert!(
        result.is_err(),
        "Expected panic from infinite float range"
    );
    println!("    -> Confirmed: panicked.");
}

fn float_range_neg_infinity_to_infinity() {
    println!("  [float_range_neg_infinity_to_infinity] f64::NEG_INFINITY..=f64::INFINITY does NOT panic.");
    // Surprisingly, this does NOT panic because the range conversion normalizes
    // the bounds. NEG_INFINITY and INFINITY get clamped/handled by the float
    // range conversion. This is worth documenting as unexpected behavior.
    let result = std::panic::catch_unwind(|| {
        let v = (f64::NEG_INFINITY..=f64::INFINITY).sample(0.5);
        v
    });
    match result {
        Ok(v) => println!("    -> Succeeded with value: {v} (no panic, possibly surprising)."),
        Err(_) => println!("    -> Panicked (range assertion triggered)."),
    }
}

fn main() {
    let exploits: &[(&str, fn())] = &[
        ("weight_nan", weight_nan),
        ("weight_infinity", weight_infinity),
        ("weight_neg_infinity", weight_neg_infinity),
        ("weight_zero", weight_zero),
        ("weight_negative", weight_negative),
        ("weight_tiny", weight_tiny),
        ("sample_nan", sample_nan),
        ("sample_infinity", sample_infinity),
        ("sample_neg_infinity", sample_neg_infinity),
        ("size_combinator_nan", size_combinator_nan),
        ("size_combinator_infinity", size_combinator_infinity),
        ("float_range_nan", float_range_nan),
        ("float_range_infinity", float_range_infinity),
        ("float_range_neg_infinity_to_infinity", float_range_neg_infinity_to_infinity),
    ];

    let args: Vec<String> = std::env::args().collect();
    let filter = args.get(1).map(|s| s.as_str());

    println!("=== Edge Case: Assertion Panics ===\n");

    let mut ran = 0;
    let mut passed = 0;
    for (name, exploit) in exploits {
        if filter.is_some_and(|f| f != *name) {
            continue;
        }
        ran += 1;
        exploit();
        passed += 1;
    }

    println!("\n--- {passed}/{ran} exploits confirmed ---");
    if passed < ran {
        process::exit(1);
    }
}
