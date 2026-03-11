//! # Edge Case: NaN and Infinity Propagation
//!
//! This example explores how NaN and infinity values propagate through the
//! generator combinator chain, leading to subtle misbehavior rather than
//! explicit panics.
//!
//! Run with: cargo run --example edge_nan
//!
//! ## Findings
//!
//! 1. `dampen_with(f64::NAN, ...)` is silently accepted without validation.
//!    The pressure parameter becomes meaningless: `depth * NaN = NaN`,
//!    `utility::f64::max(NaN, 1.0)` returns `1.0`, so `size / 1.0 = size`.
//!    The depth-based dampening is completely disabled. Only the `deepest` and
//!    `limit` hard cutoffs still work.
//!
//! 2. `dampen_with(0.0, ...)` similarly disables depth-based size reduction:
//!    `depth * 0.0 = 0.0`, `max(0.0, 1.0) = 1.0`, so `size / 1.0 = size`.
//!    Only the hard cutoffs have any effect.
//!
//! 3. `dampen_with(_, 0, _)` forces size to 0 immediately (depth >= 0 is always
//!    true), making recursive generators produce only base cases.
//!
//! 4. Negative size values (`sample(-0.5)`) are silently clamped to 0.0 by
//!    `Sizes::new`. No error is reported, but all generated values are minimal.
//!
//! 5. Size values > 1.0 (`sample(2.0)`) are silently clamped to 1.0.

use checkito::*;

fn dampen_nan_pressure() {
    println!("  [dampen_nan_pressure] dampen_with(NaN, 8, 8192) silently has no dampening effect.");

    // Build a recursive list generator.
    fn list() -> impl Generate<Item = Vec<u8>> {
        (
            with(Vec::new),
            lazy(list)
                .map(|mut v: Vec<u8>| {
                    v.push(1);
                    v
                })
                // NaN pressure: should dampen but doesn't!
                .dampen_with(f64::NAN, 8, 8192)
                .boxed(),
        )
            .any()
            .unify()
    }

    // Sample with a moderate size.
    let mut total_len = 0;
    let count = 100;
    for sample in list().samples(count) {
        total_len += sample.len();
    }
    let avg_len = total_len as f64 / count as f64;
    println!("    Average list length with NaN pressure: {avg_len:.1}");

    // Compare with normal dampening.
    fn list_normal() -> impl Generate<Item = Vec<u8>> {
        (
            with(Vec::new),
            lazy(list_normal)
                .map(|mut v: Vec<u8>| {
                    v.push(1);
                    v
                })
                .dampen() // default: pressure=1.0, deepest=8, limit=8192
                .boxed(),
        )
            .any()
            .unify()
    }

    let mut total_len_normal = 0;
    for sample in list_normal().samples(count) {
        total_len_normal += sample.len();
    }
    let avg_len_normal = total_len_normal as f64 / count as f64;
    println!("    Average list length with normal pressure: {avg_len_normal:.1}");
    println!(
        "    -> NaN pressure produces {:.1}x longer lists on average.",
        if avg_len_normal > 0.0 {
            avg_len / avg_len_normal
        } else {
            f64::INFINITY
        }
    );
}

fn dampen_zero_pressure() {
    println!("  [dampen_zero_pressure] dampen_with(0.0, 8, 8192) also silently has no dampening.");

    fn list() -> impl Generate<Item = Vec<u8>> {
        (
            with(Vec::new),
            lazy(list)
                .map(|mut v: Vec<u8>| {
                    v.push(1);
                    v
                })
                .dampen_with(0.0, 8, 8192) // Zero pressure: no dampening!
                .boxed(),
        )
            .any()
            .unify()
    }

    let mut total_len = 0;
    let count = 100;
    for sample in list().samples(count) {
        total_len += sample.len();
    }
    let avg_len = total_len as f64 / count as f64;
    println!("    Average list length with zero pressure: {avg_len:.1}");
    println!("    -> Zero pressure effectively disables size reduction per depth level.");
}

fn dampen_zero_deepest() {
    println!("  [dampen_zero_deepest] dampen_with(1.0, 0, 8192) forces immediate size=0.");

    fn list() -> impl Generate<Item = Vec<u8>> {
        (
            with(Vec::new),
            lazy(list)
                .map(|mut v: Vec<u8>| {
                    v.push(1);
                    v
                })
                .dampen_with(1.0, 0, 8192) // deepest=0: size becomes 0 immediately
                .boxed(),
        )
            .any()
            .unify()
    }

    let mut total_len = 0;
    let count = 100;
    for sample in list().samples(count) {
        total_len += sample.len();
    }
    let avg_len = total_len as f64 / count as f64;
    println!("    Average list length: {avg_len:.1}");
    println!("    -> All values are base cases (empty vecs) because size is forced to 0.");
}

fn negative_sample_size() {
    println!("  [negative_sample_size] sample(-0.5) silently clamps to size=0.");

    // Negative size is clamped to 0.0 by Sizes::new (after assertion passes
    // because -0.5 is finite and -0.5 <= -0.5).
    let v = (0u64..1_000_000).sample(-0.5);
    println!("    (0u64..1_000_000).sample(-0.5) = {v}");
    println!("    -> Value is minimal (near 0) because size is clamped to 0.0.");
    assert_eq!(v, 0, "Expected 0 since negative size clamps to 0.0");
}

fn sample_negative_one() {
    println!("  [sample_negative_one] sample(-1.0) silently clamps to size=0.");

    let v = (0u64..1_000_000).sample(-1.0);
    println!("    (0u64..1_000_000).sample(-1.0) = {v}");
    assert_eq!(v, 0, "Expected 0 since negative size clamps to 0.0");
}

fn sample_two() {
    println!("  [sample_two] sample(2.0) silently clamps to size=1.0.");

    // Size > 1.0 is clamped to 1.0 by Sizes::new.
    let v = (0u8..=255).sample(2.0);
    println!("    (0u8..=255).sample(2.0) = {v}");
    println!("    -> Value uses full range because size is clamped to 1.0.");
}

fn main() {
    println!("=== Edge Case: NaN and Infinity Propagation ===\n");

    dampen_nan_pressure();
    println!();
    dampen_zero_pressure();
    println!();
    dampen_zero_deepest();
    println!();
    negative_sample_size();
    println!();
    sample_negative_one();
    println!();
    sample_two();

    println!("\n--- All NaN/infinity propagation tests completed ---");
}
