# Missing Documentation for `State` and Its Generation Methods

## Summary

The `State` struct and most of its methods in `checkito/src/state.rs` lack inline
documentation (`///` doc comments).  `State` is a core public type in the library, and
understanding how to write custom `Generate` implementations requires knowledge of how
`State` works — its `size`, `seed`, `depth`, and `limit` fields and how each affects
generation.

## Affected Code

`checkito/src/state.rs` – the `State` struct and all its associated methods.

## Current State of Documentation

The file has no top-level module doc comment and minimal per-item documentation:

- `State` struct: no doc comment
- `State::random()`: no doc comment (used internally)
- `State::exhaustive()`: no doc comment (used internally)
- `State::fuzz()`: no doc comment (used in `state.rs` as a TODO item)
- `State::seed()`: no doc comment
- `State::size()`: no doc comment (returns a `f64` in `[0.0, 1.0]`)
- `State::scale()`: no doc comment
- `State::sizes()`: no doc comment
- `State::depth()`: no doc comment
- `State::limit()`: no doc comment
- `State::descend()`: no doc comment (increments both `depth` and `limit`)
- `State::dampen()`: no doc comment
- `State::with()`: no doc comment
- `State::bool()`, `State::u8()`, …, `State::f64()`: no doc comments
- `State::char()`: no doc comment
- `State::any_uniform()`: no doc comment
- `State::any_weighted()`: no doc comment
- `State::retry()`: no doc comment
- `State::repeat()`: no doc comment

## Why This Matters

Users implementing custom `Generate` types need to know:

1. **What `size` means**: `size` (available via `state.size()`) is in `[0.0, 1.0]` and
   controls whether to produce "small" or "large" values.  For example, `state.u32(0..=100)`
   with `size ≈ 0.0` tends to produce values near 0, while `size ≈ 1.0` produces values near
   100.

2. **What `depth` means**: `depth` is incremented by [`State::descend()`] (used in
   `Flatten`) and reset when the `With` guard drops.  It represents the current recursion
   depth and is used by `Dampen` to reduce the size of nested structures.

3. **What `limit` means**: `limit` is a cumulative count of all `descend()` calls across the
   entire generation tree.  Unlike `depth`, it is not reset.  It is used by `Dampen` with the
   `limit` parameter to stop recursive growth.

4. **How to use `state.retry()`**: For implementing `filter`-like generators, `retry` is the
   correct method to use — it repeatedly generates values until the filter predicate succeeds
   or `retries` is exhausted.

5. **How `state.repeat()` works**: For collection generators, `repeat` generates a
   sequence of values with a size chosen from `range` in a way that respects the current
   `size`.

## Proposed Fix

Add comprehensive doc comments to all public items in `state.rs`.  The level of detail should
be similar to `generate.rs`, which has excellent documentation.

### Priority Items (Most Important for Users)

1. `State` struct: Explain what it is, when users interact with it, and how fields relate.
2. `State::size()`: Explain the `[0.0, 1.0]` range and what "smaller" means.
3. `State::seed()`: Explain reproducibility.
4. `State::descend()`: Explain depth/limit tracking and its relationship with `Flatten`.
5. `State::retry()`: Explain how it's used in `Filter` implementations.
6. `State::repeat()`: Explain how it's used in collection generators.

### Example Documentation

```rust
/// The core state object passed to [`Generate::generate`].
///
/// `State` provides:
/// - A random number source (or exhaustive counter in exhaustive mode).
/// - The current `size` parameter, which guides generators to produce
///   "smaller" (closer to defaults/zero) or "larger" (closer to max) values.
/// - The current `depth`, which tracks how deeply nested the generation is
///   (for use with [`Generate::dampen`]).
/// - A cumulative `limit` that counts total descend operations.
///
/// Most users interact with `State` only when implementing custom
/// [`Generate`] types. For generating primitive values, use the provided
/// methods like [`State::u32`], [`State::bool`], etc.
pub struct State { … }

/// Returns the current generation size, in the range `[0.0, 1.0]`.
///
/// A size of `0.0` instructs generators to produce "minimal" values (e.g., `0`
/// for numbers, empty strings/vectors). A size of `1.0` instructs generators
/// to produce any value in their full range.
///
/// Generators should use `size` to scale their output appropriately.
pub fn size(&self) -> f64 { … }
```

## Impact

- **Severity:** Low (documentation gap, not a correctness issue).
- **Priority:** Medium — `State` is central to custom `Generate` implementations and
  underdocumented.
- Users writing custom generators must currently read the source code to understand how
  to use `State` correctly.

## Related Files

Also missing documentation:

- `Sizes` struct in `state.rs`: the three fields (`start`, `end`, `scale`) are not documented.
- `Weight<G>` struct: the `weight` field semantics (positive float, relative probability) are
  not documented.
- `Modes` struct: undocumented.
