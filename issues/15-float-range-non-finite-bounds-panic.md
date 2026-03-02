# Float Range Generation Panics When Range Bounds Are Non-Finite

## Summary

The float generation function in `state.rs` has a hard `assert!` (not `debug_assert!`) that
panics if the range bounds are not finite.  This means any user-provided range that includes
`INFINITY`, `NEG_INFINITY`, or `NaN` as a bound will cause a panic at runtime, even in
release builds.

## Affected Code

`checkito/src/state.rs` – inside the `floating!` macro (approximately line 730):

```rust
fn generate(state: &mut State, Range(start, end): Range<$number>) -> $number {
    assert!(start.is_finite() && end.is_finite());  // <-- hard panic in release builds
    // …
}
```

## Triggering the Panic

```rust
use checkito::Generate;

// Panics! 0.0 is finite, but INFINITY is not.
(0.0f32..=f32::INFINITY).check(|x| x < 1e30);

// Also panics:
(-f64::INFINITY..=f64::INFINITY).check(|x| x.is_finite());
```

Both of the above call `State::f32()` / `State::f64()` with a range containing a non-finite
bound, triggering the assertion.

## Why This Is a Problem

1. **Unexpected panic in user code.** The `check` and `checks` methods are documented as
   "running property tests."  A user who writes `(0.0..=INFINITY).check(…)` has a reasonable
   intention (test properties over the upper half of the float line), and a panic is not a
   useful or expected response.

2. **The panic fires in release mode.**  `assert!` (without the `debug_` prefix) is
   unconditional.  A property test suite that relies on this should use `debug_assert!` for
   invariants that are validated by construction, or should handle the non-finite case
   gracefully.

3. **Inconsistency with the `Full<f32>` generator.**  `f32::generator()` (backed by
   `Full<f32>`) internally creates ranges like `Range(f32::MIN, f32::MAX)` which are finite
   — it never calls `State::f32()` with non-finite bounds.  But nothing in the type system
   prevents a user from constructing a range with non-finite bounds.

## Root Cause

The float generation algorithm assumes finite bounds so that it can compute `end - start` and
similar differences.  Non-finite values would make these calculations undefined or NaN.

## Fix Plan

### Option A – Clamp range bounds to finite values at conversion time

Modify the `From<ops::RangeInclusive<f32>> for Range<f32>` (and similar) conversion to clamp
`INFINITY` to `MAX` and `-INFINITY` to `MIN`:

```rust
ranges!(
    f32,
    utility::f32::next_up,
    utility::f32::next_down
);

// Override the RangeFull / RangeFrom conversions to clamp infinities:
impl From<ops::RangeFull> for Range<f32> {
    fn from(_: ops::RangeFull) -> Self {
        Range(f32::MIN, f32::MAX)
    }
}
```

For user-provided ranges with explicit infinity bounds, add clamping in the `From` impl:

```rust
impl From<ops::RangeInclusive<f32>> for Range<f32> {
    fn from(r: ops::RangeInclusive<f32>) -> Self {
        let start = r.start().clamp(f32::MIN, f32::MAX);
        let end = r.end().clamp(f32::MIN, f32::MAX);
        // handle NaN: replace with 0.0 or some sentinel
        let start = if start.is_nan() { f32::MIN } else { start };
        let end = if end.is_nan() { f32::MAX } else { end };
        Range(start.min(end), start.max(end))
    }
}
```

However, this changes the semantics: `(0.0..=INFINITY).check()` would silently behave as
`(0.0..=MAX).check()`.

### Option B – Convert `assert!` to `debug_assert!` (minimal change)

Change the hard assertion to a debug assertion:

```rust
fn generate(state: &mut State, Range(start, end): Range<$number>) -> $number {
    debug_assert!(start.is_finite() && end.is_finite());
    // …
}
```

In debug builds, the violation is still caught.  In release builds, using non-finite bounds
would produce NaN values due to arithmetic on infinities, which is incorrect but at least
doesn't crash.  This approach requires documenting the precondition clearly.

### Option C – Gracefully handle non-finite range bounds

Add a specific case for non-finite bounds in `generate()`:

```rust
fn generate(state: &mut State, Range(start, end): Range<$number>) -> $number {
    // Clamp non-finite bounds to the representable finite range.
    let start = if start.is_finite() { start } else if start.is_sign_negative() { $type::MIN } else { $type::MAX };
    let end = if end.is_finite() { end } else if end.is_sign_negative() { $type::MIN } else { $type::MAX };
    let (start, end) = if start <= end { (start, end) } else { (end, start) };
    // … rest of generation
}
```

### Recommended

**Option C** is the most user-friendly approach.  Document the clamping behavior clearly so
users know that INFINITY bounds are silently treated as `MAX`/`MIN`.

## Impact

- **Severity:** Medium – can cause hard panics in user code for reasonable inputs.
- All float range generators, including `collect()` with float elements, are affected.

## Test Cases to Add

```rust
#[test]
fn f32_range_with_infinity_bound_does_not_panic() {
    // Should produce values in [0.0, MAX] without panicking.
    let result = (0.0f32..=f32::INFINITY).check(|x| x.is_finite());
    // May fail (since MAX is finite but not all values in range are), but should not panic.
    let _ = result;
}

#[test]
fn f64_range_full_infinity_does_not_panic() {
    let result = (-f64::INFINITY..=f64::INFINITY).check(|_| true);
    assert!(result.is_none());
}
```
