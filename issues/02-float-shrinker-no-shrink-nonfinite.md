# Float Shrinker Does Not Shrink Non-Finite Values

## Summary

When a floating-point property test fails with a non-finite value (`f32::INFINITY`,
`f32::NEG_INFINITY`, `f64::INFINITY`, `f64::NEG_INFINITY`, or `NaN`), the shrinker
returns `None` immediately without attempting any shrinking.  As a result, the final
failure report always shows the raw non-finite value rather than a simpler minimal
counterexample (which would usually be `0.0`, `f32::MAX`, or similar finite values).

## Affected Code

`checkito/src/primitive.rs` – `Shrink for Shrinker<$type>` inside the `floating!` macro
(currently approximately line 804–811):

```rust
fn shrink(&mut self) -> Option<Self> {
    if self.item.is_finite() {
        shrink!(self, $type)
    } else {
        None   // <-- non-finite values are never shrunk
    }
}
```

## Why This Is a Problem

The `Full<f32>` and `Full<f64>` generators include non-finite special values via `SPECIAL`:

```rust
const SPECIAL: SpecialType = Any((
    0 as $type, $type::MIN, $type::MAX, $type::EPSILON,
    $type::INFINITY, $type::NEG_INFINITY, $type::MIN_POSITIVE, $type::NAN
));
```

A property that fails for `INFINITY` (e.g., something like `value.is_finite()` returning
`false`) will be reported with `INFINITY` as the minimal counterexample.  The shrinker
makes no attempt to find a smaller failing value, even though finite values like `f32::MAX`
might also fail the same property (and are "simpler" in the sense of being more useful for
debugging).

### Example

```rust
use checkito::*;

let fail = f32::generator().check(|x| x.is_finite()).unwrap();
// Current behavior: fail.item may be INFINITY, NEG_INFINITY, or NaN — no shrinking.
// Expected behavior: shrinker should attempt e.g. MAX → (MAX+MIN)/2 → ... → 0.0
// or at minimum try MAX, 1.0, 0.0 as candidates.
```

## Expected Shrinking Behavior for Non-Finite Values

A reasonable shrinking strategy for non-finite floats would mirror the pattern used for
finite floats but with an initial step that "normalizes" the value:

| Initial value   | Step 1 to try | Then             |
|-----------------|---------------|------------------|
| `INFINITY`      | `MAX`         | binary search → 0|
| `NEG_INFINITY`  | `MIN`         | binary search → 0|
| `NaN`           | `0.0`         | (done)           |

This matches how other property-testing libraries handle non-finite floats and is consistent
with the principle that shrinking should move values toward `0.0`.

## Proposed Fix

Modify the `Shrink for Shrinker<$type>` implementation inside the `floating!` macro to
handle non-finite values with an initial transition to a finite value:

```rust
fn shrink(&mut self) -> Option<Self> {
    if self.item.is_finite() {
        shrink!(self, $type)
    } else if self.item.is_nan() {
        // NaN → try 0.0 (the simplest finite value)
        Some(Shrinker {
            start: 0.0 as $type,
            end: 0.0 as $type,
            item: 0.0 as $type,
            direction: Direction::None,
        })
    } else if self.item == $type::INFINITY {
        // +∞ → try MAX (finite upper bound), then binary-search toward 0
        Some(Shrinker {
            start: self.start.min($type::MAX),
            end: $type::MAX,
            item: $type::MAX,
            direction: Direction::None,
        })
    } else {
        // -∞ → try MIN (finite lower bound), then binary-search toward 0
        Some(Shrinker {
            start: $type::MIN,
            end: self.end.max($type::MIN),
            item: $type::MIN,
            direction: Direction::None,
        })
    }
}
```

Note: the `shrink!` macro uses the `item` field to determine the direction of shrinking (low
or high), so setting `item` to `MAX`/`MIN` and leaving `start`/`end` as the original range
bounds allows the existing binary-search logic to take over after the first non-finite → finite
transition.

## Impact

- **Correctness:** Properties failing due to non-finite float values will not be minimized.
  Users see a harder-to-debug failure report.
- **Severity:** Medium – affects float properties explicitly testing for non-finite behavior
  and any property using `f32::generator()` / `f64::generator()` (which include non-finite
  values in their special cases).

## Test Cases to Add

```rust
#[test]
fn float_infinity_is_shrunk_to_finite_value() {
    // A property that fails for any value > 0 should shrink 
    // INFINITY to something finite and minimal.
    use checkito::primitive::Shrinker;
    let mut shrinker = Shrinker::<f32> {
        start: f32::MIN,
        end: f32::MAX,
        item: f32::INFINITY,
        direction: checkito::primitive::Direction::None,
    };
    // First shrink step should produce a finite value.
    let next = shrinker.shrink().expect("INFINITY should be shrinkable");
    assert!(next.item().is_finite(), "first shrink of INFINITY must be finite");
}

#[test]
fn float_neg_infinity_is_shrunk_to_finite_value() {
    use checkito::primitive::Shrinker;
    let mut shrinker = Shrinker::<f64> {
        start: f64::MIN,
        end: f64::MAX,
        item: f64::NEG_INFINITY,
        direction: checkito::primitive::Direction::None,
    };
    let next = shrinker.shrink().expect("NEG_INFINITY should be shrinkable");
    assert!(next.item().is_finite());
}

#[test]
fn float_nan_is_shrunk_to_zero() {
    use checkito::primitive::Shrinker;
    let mut shrinker = Shrinker::<f32> {
        start: f32::MIN,
        end: f32::MAX,
        item: f32::NAN,
        direction: checkito::primitive::Direction::None,
    };
    let next = shrinker.shrink().expect("NaN should be shrinkable");
    assert_eq!(next.item(), 0.0f32);
}
```
