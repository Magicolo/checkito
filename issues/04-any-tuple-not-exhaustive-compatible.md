# Issue: any() on Tuples Does Not Use Exhaustive Mode Correctly

## Summary

The `Any<(G0, G1, ...)>` generator (tuple variant) dispatches to one of its
sub-generators by drawing a random `u8` index. In exhaustive mode this
behaviour is identical to random mode, so not all sub-generators are visited
deterministically and the exhaustive coverage guarantee is not upheld.

The weighted tuple variant (`(Weight<G0>, Weight<G1>, ...)`) has the same
problem, using a float draw instead.

## Location

- `checkito/src/any.rs:188-196` — unweighted tuple `Any` impl
- `checkito/src/any.rs:216-230` — weighted tuple `(Weight<G0>, ...)` impl

Both carry TODO comments:

```rust
// TODO: In exhaustive mode, one can determine which generator
// to use with the current exhaustive index.
// See `State::any_exhaustive`.
```

```rust
// TODO: In exhaustive mode, the state will try to cover all possible
// floats between '0.0..=_total' and some generators may remain uncovered.
// Instead, do something similar as `State::any_exhaustive`.
```

## Root Cause

```rust
fn generate(&self, state: &mut State) -> Self::Shrink {
    // Uses a random u8 even in exhaustive mode:
    let value = state.with().size(1.0).u8(..N_GENERATORS);
    match value {
        0 => orn::Or2::T0(self.0.0.generate(state)),
        1 => orn::Or2::T1(self.0.1.generate(state)),
        _ => unreachable!(),
    }
}
```

The slice-based `any([G0, G1, ...])` already calls `State::any_exhaustive`
(state.rs:217) which deterministically maps the index to the correct
sub-generator. The tuple-based `Any<(G0, G1, ...)>` should do the same.

## Impact

- `(gen_a, gen_b).any()` in exhaustive mode produces values from `gen_a` and
  `gen_b` with equal probability (50/50), rather than interleaving them
  deterministically.
- For short runs, one sub-generator may never be exercised at all.
- The `CARDINALITY` reported (sum of sub-generator cardinalities) is correct,
  but the runtime behaviour does not match that contract.
- The weighted variant ignores weights in exhaustive mode (which is intentional
  for slice-based `any`) but also fails to provide deterministic coverage.

## Proposed Fix

Reuse `State::any_exhaustive` in the tuple `generate` impl, passing an array
or iterator of sub-generators (analogous to the slice-based path):

```rust
fn generate(&self, state: &mut State) -> Self::Shrink {
    match &mut state.mode {
        Mode::Exhaustive(index) => {
            // Deterministically pick based on exhaustive index.
            let gens: [&dyn Generate<Item=..., Shrink=...>; N] = [...];
            match State::any_exhaustive(index, gens) {
                Some(gen) => gen.generate(state),
                None => unreachable!(),
            }
        }
        _ => { /* existing random branch */ }
    }
}
```

The challenge is that each sub-generator produces a different concrete `Shrink`
type, so they must be wrapped in `orn::Or` variants. The macro-generated code
needs to construct a virtual dispatch table similar to what is done for
`any`.

For the weighted variant, weights should be ignored in exhaustive mode (same
as the slice-based path).

## Investigation Required

The existing `any_exhaustive` function is designed for homogeneous iterators.
For the tuple variant, each element has a different type. The fix requires
either:
1. Allocating a small array of `Box<dyn Generate<...>>` (requires boxing), or
2. Directly replicating the `any_exhaustive` index-decomposition logic inline
   in the macro, applied to each arm's cardinality.

Option 2 avoids allocations and is preferable for performance.

## Testing Strategy

1. `(0u8..=1, 10u8..=11).any()` in exhaustive mode with `count = 4` should
   produce all four values: one from each of `{0,1}` and one from each of
   `{10,11}`.
2. The order should be deterministic for a given seed / exhaustive index.
3. Regression: random mode should be unaffected.

## Related

- `checkito/src/any.rs:188-196, 216-230` (TODO comments)
- `checkito/src/state.rs:116-130` (`any_exhaustive` implementation)
- Issue 02 (Full<T> not exhaustive compatible) — same class of problem
