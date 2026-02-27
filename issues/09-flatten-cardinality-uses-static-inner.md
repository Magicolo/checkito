# `Flatten::cardinality()` Returns Inaccurate Dynamic Cardinality

## Summary

The dynamic `cardinality()` method on `Flatten<G>` uses the **static** inner cardinality
(`I::CARDINALITY`) rather than a runtime estimate.  When the inner generator's dynamic
cardinality differs from its static cardinality (e.g., for range generators), the returned
value is incorrect, which can cause the library to incorrectly switch between exhaustive and
random generation modes.

## Affected Code

`checkito/src/flatten.rs` – the `Generate for Flatten<O>` implementation:

```rust
impl<I: Generate, O: Generate<Item = I> + ?Sized> Generate for Flatten<O> {
    type Item = I::Item;
    type Shrink = Shrinker<I::Shrink, O::Shrink>;

    const CARDINALITY: Option<u128> = cardinality::all_product(O::CARDINALITY, I::CARDINALITY);

    fn generate(&self, state: &mut State) -> Self::Shrink { … }

    fn cardinality(&self) -> Option<u128> {
        cardinality::all_product(self.0.cardinality(), I::CARDINALITY)  // <-- uses static I
    }
}
```

The `cardinality()` method calls `self.0.cardinality()` (correct, dynamic outer cardinality)
but then multiplies by `I::CARDINALITY` (the **static** inner cardinality for the type `I`).

## Why This Matters

Consider:

```rust
// Outer generator: a single u8 value 5 (cardinality = 1)
// Inner generator: range 0..=5 (cardinality = 6, but static u8 cardinality = 256)
let gen = same(5u8).flat_map(|v| 0..=v);
gen.cardinality()
// Current: all_product(Some(1), u8::CARDINALITY) = all_product(Some(1), Some(256)) = Some(256)
// Expected: all_product(Some(1), Some(6)) = Some(6)
```

With cardinality `Some(256)`, the library decides to use **random** generation for the
`flat_map` result (because `256 > GENERATES = 1024` is false but the specific value depends
on the test).  If the true cardinality were `Some(6)`, the library would immediately use
**exhaustive** generation.

More concretely, if `GENERATES = 1024` and the static inner cardinality happens to be `≤ 1024`,
the flat_map will be exhaustively iterated — generating all 256 u8 values instead of just the
6 that actually appear. Conversely, if the static inner cardinality is larger than `GENERATES`,
random mode is used even if the actual value space is tiny.

## Root Cause

Computing the true dynamic cardinality of a `Flatten` requires knowing what the outer
generator will produce at runtime, which is not possible at the time `cardinality()` is
called.  The static cardinality `I::CARDINALITY` is used as an upper bound.

This is a fundamental limitation, not a simple bug — but the current behavior should be
documented and, where possible, improved.

## Possible Improvements

### Option A – Document the known limitation (minimal)

Add a doc comment to `Flatten::cardinality()` explaining that the dynamic cardinality is an
overestimate:

```rust
/// Returns a conservative upper bound on the cardinality of this flattened generator.
///
/// Because the actual cardinality of the inner generator depends on the runtime value
/// produced by the outer generator, the static inner cardinality (`I::CARDINALITY`) is
/// used as an upper bound.  This may cause the checker to underestimate how many unique
/// values this generator can produce, potentially triggering random mode when exhaustive
/// mode would be more accurate.
fn cardinality(&self) -> Option<u128> {
    cardinality::all_product(self.0.cardinality(), I::CARDINALITY)
}
```

### Option B – Return `None` when inner cardinality is unknowable

If the static inner cardinality is not known at compile time, return `None` rather than
`None` × outer, to avoid overestimates:

```rust
fn cardinality(&self) -> Option<u128> {
    match I::CARDINALITY {
        Some(_) => cardinality::all_product(self.0.cardinality(), I::CARDINALITY),
        None => None,
    }
}
```

This is already what the `const CARDINALITY` does, so this is effectively the same as the
current behavior but with the documentation accurately describing the semantics.

### Option C – Sampling-based estimate (not recommended)

The only way to get a better dynamic estimate is to sample the outer generator many times and
compute cardinality of observed inner generators.  This is expensive and nondeterministic;
not recommended.

### Recommended

**Option A** — document the limitation in the `cardinality()` method.  The current behavior
(using static inner cardinality) is a reasonable fallback, but users and future maintainers
should understand why exhaustive mode for `flat_map` generators may be less accurate than for
primitive generators.

## Impact

- **Severity:** Low-medium.  The library still produces correct test results; only the
  automatic exhaustive-vs-random mode selection is affected.
- Users who explicitly set `checker.generate.exhaustive = Some(true)` are unaffected.

## Test Cases to Add

```rust
#[test]
fn flat_map_cardinality_is_conservative_but_documented() {
    // Outer: single value 5 (cardinality 1)
    // Inner: 0..=5 (6 values, but static u8 cardinality is 256)
    let gen = same(5u8).flat_map(|v| 0u8..=v);
    let c = gen.cardinality();
    // The dynamic cardinality is an overestimate; we at minimum verify it is Some.
    assert!(c.is_some());
    // It should NOT be less than the actual number of values (6).
    assert!(c.unwrap() >= 6);
}
```
