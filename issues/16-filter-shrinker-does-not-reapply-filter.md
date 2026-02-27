# `filter::Shrinker::shrink()` Does Not Re-Apply the Filter to Candidate Values

## Summary

The `Shrink for filter::Shrinker<S, F>` implementation in `checkito/src/filter.rs` does not
check whether the shrunk value still passes the filter predicate when producing candidate
shrinkers.  This means the shrinking phase can produce candidates whose `item()` value
returns `None` (filtered out), even when the original failing value was `Some(_)`.

## Affected Code

`checkito/src/filter.rs` – `Shrink for Shrinker<S, F>`:

```rust
impl<S: Shrink, F: Fn(&S::Item) -> bool + Clone> Shrink for Shrinker<S, F> {
    type Item = Option<S::Item>;

    fn item(&self) -> Self::Item {
        let item = self.shrinker.as_ref()?.item();
        if (self.filter)(&item) {
            Some(item)
        } else {
            None              // <-- correct: filtered items become None
        }
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Shrinker {
            filter: self.filter.clone(),
            shrinker: Some(self.shrinker.as_mut()?.shrink()?),
            // The filter is NOT applied to check if the shrunk value matches.
        })
    }
}
```

## Why This Matters

Consider a filter generator: `(0u8..=100).filter(|&x| x % 2 == 0)`.  When this generator
produces `Some(42)` and the test fails, the shrinker tries to find a simpler even number.
During shrinking:

1. `shrink()` is called on the `filter::Shrinker` wrapping the `u8` shrinker for value `42`.
2. The inner `u8` shrinker might produce the candidate value `21` (odd — doesn't pass the
   filter).
3. `item()` on this candidate returns `None` (since `21 % 2 != 0`).
4. The test is evaluated with `None`.

If the test accepts `None` (e.g., `|x: Option<u8>| x.map_or(true, |v| v < 50)`), the
shrinking stops treating `None` as a "passing" input (the inner shrinker is discarded) even
though we might find a better even candidate.

This **doesn't cause incorrect test results** (the library correctly reports a failure or
success for each candidate), but it can cause **poor shrinking quality**: the minimal
counterexample might be `None` or a large even number instead of `Some(0)` or the smallest
even number that fails.

## Concrete Example

```rust
// Test: all even numbers less than 50 should pass; some even ≥ 50 should fail.
let fail = (0u8..=100)
    .filter(|&x| x % 2 == 0)
    .check(|x: Option<u8>| x.map_or(true, |v| v < 50))
    .unwrap();

// Current behavior: fail.item may be Some(50) or None, depending on which
// intermediate odd values were produced during shrinking.
// Expected: fail.item should be Some(50) (the smallest even number ≥ 50 that fails).
```

## Root Cause

The `shrink()` method delegates entirely to the inner shrinker without filtering.  The
filter is only applied in `item()`.  The check engine receives `None` as the item and
evaluates the property with `None`, which affects shrinking trajectory.

## Proposed Fix

The filter should be applied during `shrink()` to skip over candidates that fail the
predicate:

```rust
fn shrink(&mut self) -> Option<Self> {
    loop {
        let inner = self.shrinker.as_mut()?.shrink()?;
        let item = inner.item();
        if (self.filter)(&item) {
            return Some(Shrinker {
                filter: self.filter.clone(),
                shrinker: Some(inner),
            });
        }
        // Candidate doesn't match the filter; try the next shrunk value.
        *self.shrinker.as_mut()? = inner;
    }
}
```

### Concern: Infinite Loop

If no shrunk value ever matches the filter (e.g., the value is at the boundary and all
smaller values are filtered out), this loop would spin until the inner shrinker returns
`None`.  The inner shrinker will eventually return `None` (it has finite descent steps),
so the loop terminates.

### Alternative: Keep Current Behavior, Document It

The current behavior of producing `None` candidates is actually useful in some cases: it
allows the checker to discover whether the failure is due to the value itself or due to the
filter constraint.  If the test accepts `None`, the "failure" is specific to the value; if
it rejects `None`, the constraint is part of the failure condition.

If this is the intended design, add a documentation comment explaining why unfiltered
candidates are produced during shrinking.

## Impact

- **Severity:** Low – does not produce incorrect test failures, but can reduce the quality of
  shrunk counterexamples when using `filter`.
- The issue is more visible for tight filter predicates (few values pass) and complex
  properties.

## Test Cases to Add

```rust
#[test]
fn filter_shrinks_to_smallest_matching_value() {
    // The smallest even number ≥ 50 is 50 itself.
    let fail = (0u8..=100)
        .filter(|&x| x % 2 == 0)
        .check(|x: Option<u8>| x.map_or(true, |&v| v < 50))
        .unwrap();
    assert_eq!(fail.item, Some(50u8));
}
```
