# `all::shrink` Resets Index When No More Elements Can Be Shrunk

## Summary

The `all::shrink` helper function in `checkito/src/all.rs` uses `index` as both an index
into the shrinkers array and a "progress indicator."  When an element at position `*index`
cannot be shrunk, `*index` is incremented.  However, this mutation of `*index` in the caller
means that once all elements fail to shrink, `*index` is left pointing past the end of the
array, and the `all::Shrinker` (for arrays, slices, and tuples) never tries shrinking earlier
elements again.

## Background

```rust
pub(crate) fn shrink<S: Shrink, I: AsMut<[S]> + Clone>(
    shrinkers: &mut I,
    index: &mut usize,
) -> Option<I> {
    loop {
        let old = shrinkers.as_mut().get_mut(*index)?;
        if let Some(new) = old.shrink() {
            let mut shrinkers = shrinkers.clone();
            shrinkers.as_mut()[*index] = new;
            break Some(shrinkers);
        } else {
            *index += 1;   // advance to next element
        }
    }
}
```

When `all::shrink` is called by the check engine in a shrinking loop:

```rust
// In all::Shrinker<[S; N]>
fn shrink(&mut self) -> Option<Self> {
    let shrinkers = shrink(&mut self.shrinkers, &mut self.index)?;
    Some(Self { shrinkers, index: self.index })
}
```

The `self.index` is permanently incremented until it goes past the end. Once `index >= N`
(or `>= slice.len()`), `all::shrink` returns `None` forever — no more shrinking happens,
even though earlier elements in the array (index 0..self.index) might have further shrink
candidates.

## Concrete Example

For a failing 3-element array `[100u8, 200u8, 150u8]`:

1. Shrink element 0: `100 → 50 → 25 → ... → 0` (reaches minimum)
2. Advance `index = 1`, shrink element 1: `200 → 100 → ... → 0` (reaches minimum)
3. Advance `index = 2`, shrink element 2: `150 → 75 → ... → 0` (reaches minimum)
4. `index = 3`, `get_mut(3) = None` → `shrink()` returns `None` → **done**.

But what if in step 1, element 0 was shrunk from 100 to 50, and NOW element 1 could be
shrunk further (e.g., from 200 to some value that wasn't reachable before)?  The current
design doesn't retry elements at lower indices.

## Why This Is By Design (Mostly)

The "one pass, left-to-right" shrinking strategy is intentional and common in property-testing
libraries.  It is fast (O(N) passes through the array) and good enough for most cases.
Shrinking individual elements independently without restarting is a deliberate trade-off.

## Actual Bug: `index` Returned in `Shrinker`

The real issue is in how the `index` is included in the candidate shrinker:

```rust
fn shrink(&mut self) -> Option<Self> {
    let shrinkers = shrink(&mut self.shrinkers, &mut self.index)?;
    Some(Self {
        shrinkers,
        index: self.index,   // <-- uses the ALREADY-INCREMENTED index
    })
}
```

If the check engine accepts the candidate shrinker (`Shrinker { shrinkers, index: N }`),
and then calls `shrink()` on it, it immediately tries `get_mut(N)` which fails → returns
`None`.  The candidate shrinker can **never be shrunk further**.

The correct behavior would be: if the check engine accepts a candidate at position `N`, the
new "current" shrinker should try to shrink element `N` further, not start at `N+1`.

### Correct Fix

The candidate shrinker should have `index = self.index` (the current position), NOT the
incremented one:

```rust
fn shrink(&mut self) -> Option<Self> {
    let current_index = self.index;
    let shrinkers = shrink(&mut self.shrinkers, &mut self.index)?;
    Some(Self {
        shrinkers,
        index: current_index,   // restart from current position in the candidate
    })
}
```

Wait, but this might cause infinite loops if the candidate is accepted and `shrink()` is
called again (it would try to shrink the same element again from the same index). Actually,
since `shrinkers[current_index]` is now `new_shrinker` (already partially shrunk), calling
`new_shrinker.shrink()` will return a further-shrunk value or `None`.  This is correct.

But `self.index` was advanced by `shrink()` to skip elements that couldn't be shrunk.  So
in `all::shrink`, `*index` is left at the position of the element that WAS shrunk.  The
candidate's `index = current_index` means it tries to shrink the same element (now already
partially shrunk) next time.

## Ambiguity in Intended Behavior

It's unclear whether the current behavior is:
1. **Intentional**: Shrink each element once then move on (fast but incomplete).
2. **Unintentional**: Accidentally skips re-shrinking the current element after a successful
   shrink step.

Looking at the implementation: `shrink(&mut self.shrinkers, &mut self.index)` **does not
advance `self.index`** on success — it only advances on failure.  So after a successful
shrink, `self.index` stays at the same position.  The candidate shrinker gets `index:
self.index`, which is the same position, so it WILL try shrinking the same element further.

This means the current behavior is actually correct for iterative shrinking of one element:
- Success: candidate has same `index`, next call tries to shrink element `index` further.
- Failure: `index` incremented, next call tries element `index + 1`.

## Re-Assessment

On further analysis, the behavior IS correct:

1. `all::shrink` advances `*index` only on failure.
2. The candidate shrinker has `index: self.index` (same position).
3. On success (failure propagated to test), the candidate is accepted and continues
   shrinking element `index`.

So the "bug" described above is not actually present.  The analysis reveals a **documentation
gap**: the `all::shrink` function and `Shrinker::shrink()` methods would benefit from
comments explaining this left-to-right, one-element-at-a-time strategy.

## Remaining Issue: Missing `shrinks()` Field Semantics Documentation

The `Pass::shrinks()` and `Fail::shrinks()` methods return the number of shrink iterations,
but the documentation doesn't clearly state:

1. Whether this counts "successful shrinks" (test-failing candidates accepted) or "total
   shrink attempts."
2. What "0 shrinks" means (the original failing value was not further reduced).

This should be clarified.

## Fix Plan

1. Add a doc comment to `all::shrink` explaining the left-to-right shrinking strategy and
   why `*index` is advanced on failure but not on success.
2. Clarify `Pass::shrinks()` and `Fail::shrinks()` documentation.

## No Code Change Required

After closer analysis, the core shrinking logic in `all.rs` appears to be correct.  The
primary action item is improved documentation.
