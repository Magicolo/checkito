//! # Red Team Report: checkito Public API Edge Cases
//!
//! This report summarizes all findings from attempting to break the `checkito`
//! library through its public API. No unsafe code or external dependencies were
//! used. Each finding has a corresponding example in the `examples/` folder.
//!
//! ## Summary
//!
//! | # | Category | Severity | Description | Example File |
//! |---|----------|----------|-------------|--------------|
//! | 1 | Panic | High | `Weight::new(NaN/Inf/0.0)` panics with opaque assert | edge_panic |
//! | 2 | Panic | High | `sample(NaN/Inf)` panics via `Sizes::new` assertion | edge_panic |
//! | 3 | Panic | High | `size(\|_\| NaN)` panics via `Sizes::new` assertion | edge_panic |
//! | 4 | Panic | High | Float ranges with NaN endpoints panic | edge_panic |
//! | 5 | Panic | Medium | `collect_with(usize::MAX)` panics with capacity overflow | edge_overflow |
//! | 6 | Silent | Medium | `dampen_with(NaN, ...)` silently disables dampening | edge_nan |
//! | 7 | Silent | Medium | `dampen_with(0.0, ...)` silently disables dampening | edge_nan |
//! | 8 | Silent | Low | Negative size values silently clamped to 0.0 | edge_nan |
//! | 9 | Logic | High | `cardinality::<_, 0>()` makes checks pass vacuously | edge_cardinality |
//! | 10 | Logic | Medium | `cardinality::<_, 1>()` limits checks to one iteration | edge_cardinality |
//! | 11 | Logic | Medium | Lying about cardinality limits exhaustive coverage | edge_cardinality |
//! | 12 | Resource | Low | Regex with huge repeats allocates huge strings | edge_overflow |
//! | 13 | Behavior | Info | `Any` over empty slice always produces `None` | edge_empty |
//! | 14 | Behavior | Info | `f64::NEG_INFINITY..=f64::INFINITY` works (no panic) | edge_panic |
//!
//! ---
//!
//! ## Detailed Findings
//!
//! ### 1. Assertion Panics from `Weight::new` (HIGH)
//!
//! **File**: `state.rs:90-91`
//!
//! `Weight::new(weight, generator)` uses bare `assert!` macros to validate the
//! weight parameter. This means:
//! - `Weight::new(f64::NAN, gen)` → `assertion failed: weight.is_finite()`
//! - `Weight::new(f64::INFINITY, gen)` → `assertion failed: weight.is_finite()`
//! - `Weight::new(0.0, gen)` → `assertion failed: weight >= f64::EPSILON`
//! - `Weight::new(-1.0, gen)` → `assertion failed: weight >= f64::EPSILON`
//! - `Weight::new(f64::EPSILON / 2.0, gen)` → same assertion
//!
//! The error messages provide no context about what went wrong. A user computing
//! a weight from arithmetic (e.g., `a / b` where `b = 0.0` produces NaN) will
//! see a cryptic panic.
//!
//! **Recommendation**: Use descriptive assertion messages or return `Result`.
//!
//! ### 2. Assertion Panics from `sample(NaN/Inf)` (HIGH)
//!
//! **File**: `state.rs:937`
//!
//! The `Sample::sample(size)` method converts the `size: f64` into `Sizes` via
//! `f64::into() -> Range(v, v) -> Sizes::new(v, v, SCALE)`. The `Sizes::new`
//! function asserts `start.is_finite() && end.is_finite() && start <= end`.
//!
//! This means:
//! - `gen.sample(f64::NAN)` → panic (NaN fails both `is_finite` and `<=`)
//! - `gen.sample(f64::INFINITY)` → panic (not finite)
//! - `gen.sample(f64::NEG_INFINITY)` → panic (not finite)
//!
//! A user passing an unvalidated configuration value as a size will crash.
//!
//! **Recommendation**: Clamp or validate before the assertion, or use
//! descriptive error messages.
//!
//! ### 3. Size Combinator Panics with NaN/Inf (HIGH)
//!
//! **File**: `size.rs:16`, `state.rs:937`
//!
//! `gen.size(|_| f64::NAN).sample(0.5)` panics because the closure result is
//! converted to `Sizes` via `Into`, hitting the same assertion as finding #2.
//!
//! Similarly, `gen.size(|_| f64::INFINITY)` panics.
//!
//! ### 4. Float Range NaN Panics (HIGH)
//!
//! **File**: `state.rs:720` (float generate function)
//!
//! The float generation function `generate()` asserts:
//! ```text
//! assert!(start.is_finite() && end.is_finite());
//! ```
//!
//! Any range containing NaN (`f64::NAN..=1.0`) panics during generation.
//! The `f64::INFINITY..=f64::INFINITY` case also panics (via Rust's stdlib
//! `clamp` assertion). However, `f64::NEG_INFINITY..=f64::INFINITY` does NOT
//! panic — the range conversion normalizes the bounds, and the value is
//! generated from the full finite range.
//!
//! ### 5. Capacity Overflow from `collect_with(usize::MAX)` (MEDIUM)
//!
//! **File**: `collect.rs:113-119`
//!
//! `(0u8..=1).collect_with::<_, Vec<u8>>(usize::MAX).sample(0.0)` attempts to
//! allocate a `Vec` with `usize::MAX` elements, causing a capacity overflow
//! panic. There is no upper bound on the `count` parameter passed to
//! `collect_with`.
//!
//! **Recommendation**: Document the risk or add a maximum collection size.
//!
//! ### 6. NaN Pressure Silently Disables Dampening (MEDIUM)
//!
//! **File**: `state.rs:228-237`
//!
//! `dampen_with(f64::NAN, deepest, limit)` is accepted without validation.
//! The dampening formula computes `old / max(depth * NaN, 1.0)`. Since
//! `depth * NaN = NaN` and `max(NaN, 1.0) = 1.0` (NaN loses all comparisons
//! in the custom `utility::f64::max`), the result is `old / 1.0 = old`.
//!
//! The depth-based pressure is completely disabled. Only the hard `deepest` and
//! `limit` cutoffs still prevent infinite recursion.
//!
//! **Recommendation**: Validate that `pressure` is finite and positive.
//!
//! ### 7. Zero Pressure Silently Disables Dampening (MEDIUM)
//!
//! Same mechanism as #6: `depth * 0.0 = 0.0`, `max(0.0, 1.0) = 1.0`.
//!
//! ### 8. Negative Size Silently Clamped (LOW)
//!
//! **File**: `state.rs:942`
//!
//! `gen.sample(-0.5)` and `gen.sample(-1.0)` don't panic. The `Sizes::new`
//! function accepts the negative value (it's finite and `-0.5 <= -0.5`), then
//! clamps it to `0.0` via `utility::f64::clamp(start, 0.0, 1.0)`. All generated
//! values are minimal/zero.
//!
//! Similarly, `gen.sample(2.0)` is silently clamped to `1.0`.
//!
//! This silent clamping may surprise users who expect an error for out-of-range
//! sizes.
//!
//! ### 9. Cardinality Override Enables Vacuous Passes (HIGH)
//!
//! **File**: `cardinality.rs:10`
//!
//! `cardinality::<_, 0>(gen)` tells the checker that the generator produces 0
//! possible values. The checker enters exhaustive mode with 0 iterations. The
//! property check passes vacuously — even for properties that ALWAYS fail.
//!
//! This is documented behavior ("Providing an incorrect cardinality can cause
//! unexpected behavior"), but the severity is high because it silently makes
//! a test suite pass when it should fail.
//!
//! ### 10. Cardinality Override to 1 (MEDIUM)
//!
//! `cardinality::<_, 1>(0u32..1000)` makes the checker run exactly once in
//! exhaustive mode with index 0. Properties that fail on values > 0 will
//! never be tested.
//!
//! ### 11. Lying About Cardinality (MEDIUM)
//!
//! `cardinality::<_, 5>(0u32..1000)` limits exhaustive checking to 5 iterations
//! for a 1000-value space. 99.5% of the domain is untested.
//!
//! ### 12. Regex Huge Repetition (LOW)
//!
//! `regex("a{10000}", None)` generates strings of exactly 10,000 characters.
//! Very large repetition counts could cause memory exhaustion. The `repeats`
//! parameter provides an upper bound for `*` and `+`, but explicit `{N}`
//! repetitions have no limit.
//!
//! ### 13. Empty `Any` Always Produces `None` (INFO)
//!
//! An `Any` over an empty `Vec` of generators always produces `None`. This is
//! correct behavior since there's nothing to choose from, but users may not
//! expect it.
//!
//! ### 14. Infinite Float Range Works (INFO)
//!
//! `f64::NEG_INFINITY..=f64::INFINITY` successfully generates values without
//! panicking. The range conversion normalizes infinity bounds. This is arguably
//! correct behavior, but surprising given that `f64::INFINITY..=f64::INFINITY`
//! DOES panic.
//!
//! ---
//!
//! ## What Works Well
//!
//! - **Filter/FilterMap**: Always-rejecting filters do NOT cause infinite loops.
//!   The `Option<T>` wrapper cleanly handles rejection.
//!
//! - **Empty collections**: `collect_with(0)`, `array::<0>()`, `samples(0)` all
//!   produce correct empty results.
//!
//! - **Shrinking**: The shrinking engine correctly finds minimal failing values.
//!   `check(|_| false)` on `0u32..1000` shrinks to `0`.
//!
//! - **Same generator**: `same(42)` has cardinality 1 and runs exactly once in
//!   exhaustive mode.
//!
//! - **Deeply nested flatten**: 50 levels of `flat_map(same)` don't crash.
//!
//! - **Checker config**: `generate.count=0` and `shrink.count=0` produce
//!   predictable, correct results.

fn main() {
    println!("This file is a report. Run the individual edge_* examples instead.");
    println!();
    println!("Available examples:");
    println!("  cargo run --example edge_panic        # Assertion panic exploits");
    println!("  cargo run --example edge_nan          # NaN/infinity propagation");
    println!("  cargo run --example edge_overflow     # Resource exhaustion");
    println!("  cargo run --example edge_cardinality  # Cardinality override exploits");
    println!("  cargo run --example edge_empty        # Empty/degenerate generators");
}
