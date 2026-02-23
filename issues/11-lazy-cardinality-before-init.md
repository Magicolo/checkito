# Issue: Lazy Generator Reports Wrong Cardinality Before Initialization

## Summary

`checkito::prelude::lazy(f)` returns a `Lazy<G, F>` whose `cardinality()`
method reports the *type-level* cardinality of `G` before the generator has
been initialised, rather than the *instance-level* cardinality. This can cause
the checking engine to enter exhaustive mode prematurely or with an incorrect
count, producing duplicate values or missed coverage.

## Location

- `checkito/src/lazy.rs:18` — `const CARDINALITY`
- `checkito/src/lazy.rs:24-26` — `cardinality()` instance method

```rust
const CARDINALITY: Option<u128> = G::CARDINALITY;   // type-level, always

fn cardinality(&self) -> Option<u128> {
    self.0.get().map_or(G::CARDINALITY, G::cardinality)  // falls back to type-level
}
```

## Concrete Example

```rust
let generator = lazy(|| 0u8..=10);
// Before any call that initialises the lazy:
assert_eq!(generator.cardinality(), Some(256));  // wrong! actual = 11
// The checker sees cardinality = 256 <= GENERATES (1024), switches to
// exhaustive mode with count = 256.
// But only 11 distinct values exist; 245 of those 256 runs produce duplicates.
```

This is demonstrated (and accepted) by the existing test:

```rust
// checkito/tests/cardinality.rs
fn lazy_delegates_cardinality_to_inner_range() {
    let generator = lazy(|| 0u8..=10);
    assert_eq!(generator.cardinality(), Some(256));  // intentionally wrong!
    generator.sample(1.0);
    assert_eq!(generator.cardinality(), Some(11));
}
```

## Why It Happens

The `Lazy<G, F>` type uses an `OnceLock<G>` to defer construction of the inner
generator. Until the lock is initialized (i.e. until the first call to
`generate`), there is no instance to query. The implementation falls back to
`G::CARDINALITY`, which is the cardinality of the *type* `G`, not of any
specific *value* of `G`.

For `Range<u8>` the type-level cardinality is `Some(256)` (the full range),
but a specific `Range<u8>` instance such as `0u8..=10` has cardinality `Some(11)`.

## Impact

1. **False exhaustive mode**: A `lazy(|| 0u8..=10)` passed to a checker will
   be run exhaustively with `count = 256` instead of `count = 11`, wasting
   245 test runs on duplicates.
2. **Unexpected non-exhaustive mode**: A `lazy(|| some_generator_with_large_cardinality)`
   might not enter exhaustive mode when it should.

## Proposed Fix

The most backward-compatible fix is to **force initialisation during
`cardinality()`** by calling the factory function eagerly:

```rust
fn cardinality(&self) -> Option<u128> {
    self.0.get_or_init(|| self.1()).cardinality()
}
```

This initialises the `OnceLock` on the first call to `cardinality()`, ensuring
the correct instance cardinality is returned. The downside is that `cardinality()`
now has a side effect (initialisation), which may be surprising.

An alternative is to **document the limitation** and add a note to the `lazy`
function that the cardinality before first use is a type-level upper bound.

## Testing Strategy

1. After the fix: `lazy(|| 0u8..=10).cardinality()` should return `Some(11)`
   immediately, without requiring a prior `sample()` call.
2. A checker on `lazy(|| 0u8..=10)` should enter exhaustive mode with exactly
   11 iterations.

## Related

- `checkito/src/lazy.rs:18, 24-26`
- `checkito/tests/cardinality.rs:196-201` (existing test, documents current behaviour)
