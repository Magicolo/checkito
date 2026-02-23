# Issue: Clippy Warnings in Test Files (CI Failure)

## Summary

Several test files trigger Clippy lints that are denied in CI, causing build
failures on the `main` branch. The affected lints are:

1. `clippy::reversed_empty_ranges` — tests that construct inverted ranges
   (e.g. `10u8..=0`) to exercise the library's range normalisation.
2. `clippy::redundant_closure` — tests that wrap free functions in unnecessary
   closures when passed to `lazy()`.

## Affected Files

| File | Line | Lint |
|------|------|------|
| `checkito/tests/exhaustive.rs` | 42 | `reversed_empty_ranges` |
| `checkito/tests/cardinality.rs` | 172 | `reversed_empty_ranges` |
| `checkito/tests/cardinality.rs` | 178 | `reversed_empty_ranges` |
| `checkito/tests/cardinality.rs` | 192 | `reversed_empty_ranges` |
| `checkito/tests/cardinality.rs` | 206 | `redundant_closure` |
| `checkito/tests/cardinality.rs` | 214 | `redundant_closure` |

## Root Cause

### reversed_empty_ranges

The tests verify that the library normalises inverted ranges (start > end) to
their canonical form `[min, max]`. For example:

```rust
// checkito/tests/cardinality.rs:172
assert_eq!((10u8..=0).cardinality(), Some(11));
```

Clippy warns that `10u8..=0` is empty when iterated. The warning is correct
from Rust's perspective (the standard range is empty), but the library
intentionally normalises such ranges. The lint fires because Clippy does not
know about the library's custom `From<RangeInclusive<T>> for Range<T>` impl
that swaps the bounds.

### redundant_closure

```rust
// checkito/tests/cardinality.rs:206
let generator = lazy(|| bool::generator());
```

`bool::generator` is a free function with the signature `fn() -> Full<bool>`,
so Clippy suggests passing it directly: `lazy(bool::generator)`. The closure
wrapper is redundant.

## Fix

### reversed_empty_ranges

Suppress the lint on the specific lines using `#[allow(clippy::reversed_empty_ranges)]`
and store the range in a typed binding to avoid the inline expression warning:

```rust
#[allow(clippy::reversed_empty_ranges)]
let inv: std::ops::RangeInclusive<u8> = 10u8..=0;
assert_eq!(inv.cardinality(), Some(11));
```

### redundant_closure

Remove the wrapper closure:

```rust
let generator = lazy(bool::generator);
let generator = lazy(u128::generator);
```

## Status

**Fixed** in this PR. All six clippy errors are resolved and CI should now
pass the `clippy` job.

## Related

- CI workflow: `.github/workflows/test.yml`
- `cargo hack clippy --release --all-targets --all-features -- --deny warnings`
