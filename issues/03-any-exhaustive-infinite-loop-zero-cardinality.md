# `any_exhaustive` Infinite Loop When Any Generator Has `Some(0)` Cardinality

## Summary

`State::any_exhaustive` contains a `cycle()`-based loop that will spin forever if any
generator in the collection reports a cardinality of `Some(0)`.  This causes an infinite loop
(hang) rather than an error.

## Affected Code

`checkito/src/state.rs` – `State::any_exhaustive` (approximately lines 142–155):

```rust
pub(crate) fn any_exhaustive<I: IntoIterator<Item = Option<u128>, IntoIter: Clone>>(
    index: &mut u128,
    cardinalities: I,
) -> Option<usize> {
    for (i, cardinality) in cardinalities.into_iter().enumerate().cycle() {
        match cardinality {
            Some(cardinality) if *index < cardinality => return Some(i),
            Some(cardinality) => *index -= cardinality,
            None => return Some(i),
        }
    }
    None
}
```

## Why This Loops Forever

For a generator with cardinality `Some(0)`:

1. `*index < 0` is always **false** (`u128` is unsigned, 0 ≤ any u128 value is vacuously true
   in general, but `x < 0` is never true for unsigned types).
2. `*index -= 0` leaves `*index` unchanged.
3. The loop continues to the same generator on the next cycle iteration.

Because `*index` never decreases and the only `Some(0)` arm does nothing, the function loops
forever.

## How `Some(0)` Cardinality Can Arise

Several cardinality helper functions can produce `Some(0)`:

| Source | Result |
|--------|--------|
| `cardinality::any_repeat_static::<0>(anything)` | `Some(0)` (empty slice/array of generators) |
| `cardinality::all_repeat_dynamic(Some(0), Range(1, n))` | `Some(0)` (collecting at least 1 item from a zero-cardinality generator) |
| Transitive: `any_sum(Some(0), Some(0))` | `Some(0)` |

A concrete triggering example: an `Any<[G; 0]>` has static `CARDINALITY = Some(0)`.  When
the library then checks `G::cardinality()` dynamically to decide how to exhaust values, this
zero-cardinality generator would be included in an `any_exhaustive` call if it somehow ends
up inside a composite generator that itself is placed inside an `any`.

A more practical path: `Collect<G, Range(1, 10), Vec<G::Item>>` where `G` has `cardinality()
= Some(0)` (e.g., a generator whose `cardinality()` override returns `Some(0)`) passed to
`any_uniform` in exhaustive mode.  Because `any_uniform` passes per-generator dynamic
cardinalities to `any_exhaustive`, a `Some(0)` entry causes the hang.

## Reproducer

```rust
use checkito::{generate::Generate, cardinality, state::State};

// A generator that reports zero cardinality.
struct ZeroCard;
impl Generate for ZeroCard {
    type Item = u8;
    type Shrink = u8;
    const CARDINALITY: Option<u128> = Some(0);
    fn generate(&self, _: &mut State) -> u8 { 0 }
    fn cardinality(&self) -> Option<u128> { Some(0) }
}

// Wrapping it in Any in exhaustive mode:
let mut index = 0u128;
// This hangs:
State::any_exhaustive(&mut index, [Some(0u128), Some(0u128)].into_iter());
```

## Fix Plan

### Option A – Skip zero-cardinality generators

Before using a cardinality in the loop, check for zero:

```rust
pub(crate) fn any_exhaustive<I: IntoIterator<Item = Option<u128>, IntoIter: Clone>>(
    index: &mut u128,
    cardinalities: I,
) -> Option<usize> {
    let cardinalities: Vec<_> = cardinalities.into_iter().collect();
    if cardinalities.is_empty() {
        return None;
    }
    for (i, cardinality) in cardinalities.iter().copied().enumerate().cycle() {
        match cardinality {
            Some(0) => continue,          // skip empty generators
            Some(c) if *index < c => return Some(i),
            Some(c) => *index -= c,
            None => return Some(i),
        }
    }
    None
}
```

However, if *all* generators have `Some(0)` cardinality, the outer loop still cycles forever.
A guard against this case is also needed:

```rust
// If all cardinalities are Some(0), there's nothing to select.
if cardinalities.iter().all(|c| *c == Some(0)) {
    return None;
}
```

### Option B – Collect into a Vec and detect all-zero

The cleanest fix is to collect cardinalities, filter out zeros, and if nothing remains return
`None`:

```rust
pub(crate) fn any_exhaustive<I: IntoIterator<Item = Option<u128>, IntoIter: Clone>>(
    index: &mut u128,
    cardinalities: I,
) -> Option<usize> {
    let cardinalities: Vec<_> = cardinalities.into_iter().enumerate().collect();
    if cardinalities.is_empty() {
        return None;
    }
    // If all are Some(0), there is no selectable item.
    if cardinalities.iter().all(|(_, c)| *c == Some(0)) {
        return None;
    }
    loop {
        for &(i, cardinality) in &cardinalities {
            match cardinality {
                Some(0) => continue,
                Some(c) if *index < c => return Some(i),
                Some(c) => *index -= c,
                None => return Some(i),
            }
        }
    }
}
```

Note: `Option B` changes the iteration from `cycle()` on an `IntoIter` to an explicit outer
`loop` over a collected `Vec`, preserving the O(n) per-call cost while avoiding allocation in
the common non-zero case if an optimization is desired later.

### Recommended

Use **Option B** because it is explicit about the all-zero guard and makes the logic easier
to audit.

## Additional Invariant to Document

The assumption that cardinalities passed to `any_exhaustive` are non-zero should be
documented as a precondition, or the function should be made robust to violations.

## Test Cases to Add

```rust
#[test]
fn any_exhaustive_does_not_hang_with_all_zero_cardinalities() {
    let mut index = 0u128;
    // Should return None, not hang.
    let result = State::any_exhaustive(&mut index, [Some(0u128), Some(0u128)]);
    assert_eq!(result, None);
}

#[test]
fn any_exhaustive_skips_zero_cardinality_and_selects_nonzero() {
    let mut index = 0u128;
    // Cardinality [Some(0), Some(2)]: should select index 1 (the only non-zero).
    let result = State::any_exhaustive(&mut index, [Some(0u128), Some(2u128)]);
    assert_eq!(result, Some(1));
}
```
