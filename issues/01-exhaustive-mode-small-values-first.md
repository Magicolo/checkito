# Issue: Exhaustive Mode Does Not Generate Small Values First

> **Note:** This issue is also tracked as GitHub issue #33.

## Summary

The exhaustive generation mode in `state.rs` generates values linearly across
the full range instead of prioritising small (low-magnitude) values first. This
undermines one of the core principles of property testing — simple inputs
should be discovered before complex ones — and means that a failing property
is unlikely to be detected early when the test budget is small.

## Location

- `checkito/src/state.rs:600` — `integer!` macro, `Mode::Exhaustive` branch
- `checkito/src/state.rs:662` — `floating!` macro, `Mode::Exhaustive` branch

Both locations carry the comment:
```rust
// TODO: Generate 'small' values first. Maybe use the same adjustment as Random?
```

## Root Cause

### Integer exhaustive generation (state.rs:600)

```rust
Mode::Exhaustive(index) => consume(index, start as _, end as _) as $integer,
```

`consume` maps `index` linearly into `[start, end]`. For a range `0..=1000`
with `count = 10` the generated sequence is `0, 100, 200, ..., 900`. The value
`500` has the same priority as `0` and edge cases are not explored first.

The `Random` mode (state.rs:573–598) uses a `shrink` helper that applies a
size-dependent logarithmic scale, biasing toward small magnitudes early in the
run. Exhaustive mode has no equivalent.

### Float exhaustive generation (state.rs:662)

```rust
Mode::Exhaustive(index) => utility::$number::from_bits(consume(
    index,
    utility::$number::to_bits(start) as _,
    utility::$number::to_bits(end) as _) as _),
```

Values are enumerated in total-order bit space. This gives no preference to
`0.0`, `1.0`, `EPSILON`, etc.

## Impact

- Properties that fail on small inputs (e.g. `x == 0`, `x == 1`) may not be
  caught until late in the exhaustive run, or at all when `count` is limited.
- Exhaustive mode is not equivalent to "the best possible coverage" — it is
  effectively random-linear, which is worse than the size-scaled random mode
  for discovering simple failures.

## Proposed Fix

### For integers

Map `index` through the same size-adjusted function used by `Random`:

```rust
Mode::Exhaustive(index) => {
    let count = u128::wrapping_sub(end as _, start as _).saturating_add(1);
    let size = if count <= 1 {
        1.0
    } else {
        (*index as f64 + 0.5) / count as f64   // uniform progress 0..1
    };
    // Apply the same `shrink` helper used in Random mode to bias toward small
    // magnitudes, then add to the appropriate boundary.
    ...
}
```

An alternative is to use the interleaved-zero strategy already used for
exhaustive generation of `Any` buckets:
- For non-negative ranges: `0, 1, 2, ...`
- For non-positive ranges: `0, -1, -2, ...`
- For ranges spanning zero: `0, 1, -1, 2, -2, ...`

This is predictable, deterministic, and tests the smallest magnitudes first.

### For floats

Apply the same interleaved ordering using the total-order bit mapping:
- Start from `to_bits(0.0)` and fan out in both directions.
- Alternatively, emit special values (`0.0`, `1.0`, `-1.0`, `EPSILON`, etc.)
  before entering the linear scan.

## Testing Strategy

1. Assert that exhaustive generation of `0i32..=1000` with `count = 10`
   produces values `[0, 1, -1, 2, -2, ...]` (or at least `0` first).
2. Assert that the first value of `f32::generator()` in exhaustive mode is
   `0.0` or a small finite value.
3. Regression: confirm that exhaustive mode still covers the full range when
   `count >= cardinality`.

## Related

- `checkito/src/state.rs:600` and `state.rs:662` (TODO comments)
- GitHub issue #33
