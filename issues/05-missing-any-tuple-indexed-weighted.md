# Issue: Missing any_tuple_indexed and any_tuple_weighted Implementations

## Summary

`State` exposes `any` and `any_weighted` for slice-based random
selection but has no equivalent for tuple-based selection where each element
is a different type. This forces the `Any<(G0, G1, ...)>` tuple implementation
to use a raw `u8` draw instead of the proper exhaustive-mode-aware helper.

## Location

- `checkito/src/state.rs:260`

```rust
// TODO: Implement `any_tuple_indexed` and `any_tuple_weighted`...
```

## Context

`State::any` (state.rs:210) handles homogeneous slices:
```rust
pub(crate) fn any<'a, G: Generate>(&mut self, generators: &'a [G]) -> Option<&'a G> {
    let end = generators.len().checked_sub(1)?;
    match &mut self.mode {
        Mode::Random(_) => {
            let index = self.with().size(1.0).usize(Range(0, end));
            generators.get(index)
        }
        Mode::Exhaustive(index) => Self::any_exhaustive(index, generators),
    }
}
```

There is no corresponding method for heterogeneous tuples. The result is that
`Any<(G0, G1, ...)>` (the macro-generated tuple impl in `any.rs`) uses a
direct `u8` draw and does not benefit from the exhaustive-index routing that
`any` provides. This is the root cause of the bug described in Issue 04.

## Proposed Fix

Add `any_tuple_indexed` with a const-generic count parameter, or expose the
index-decomposition logic so that callers can drive it manually:

```rust
/// Selects one of `count` generators in an exhaustive or random manner,
/// returning the selected index and (if exhaustive) the remainder index.
pub(crate) fn any_selector(&mut self, count: usize, total: Option<u128>) -> usize {
    match &mut self.mode {
        Mode::Random(_) => self.with().size(1.0).usize(Range(0, count - 1)),
        Mode::Exhaustive(index) => {
            // Use cardinality-aware routing if total is known.
            // Otherwise fall back to a modulo split.
            todo!()
        }
    }
}
```

The callers in `any.rs` can then switch on the returned index to dispatch to
the correct `Or` arm.

## Investigation Required

The exact API surface depends on how the macro-generated tuple code can be
refactored. The key constraint is that each arm of the tuple match has a
different concrete type, so the helper cannot return a trait object without
additional boxing. A possible design:

```rust
/// Returns (selected_arm_index, updated_state_for_sub_generate).
pub(crate) fn any_tuple<const N: usize>(
    &mut self,
    cardinalities: [Option<u128>; N],
) -> usize { ... }
```

The macro then uses the returned arm index to select which sub-generator to
call.

## Related

- `checkito/src/state.rs:260` (TODO comment)
- Issue 04 (any-tuple not exhaustive compatible) — depends on this fix
