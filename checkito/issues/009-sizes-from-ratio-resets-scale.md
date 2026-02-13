# Issue: `Sizes::from_ratio` resets `scale` to default instead of preserving configured value

## Summary
`State::random` uses `Sizes::from_ratio(index, count, sizes)` to derive a per-sample size during generation. The `from_ratio` implementation currently reconstructs `Sizes` with `Self::SCALE` (the hardcoded default) rather than reusing `size.scale()`.

As a result, any non-default `Sizes` scale configured by users (or by internal combinators) is silently dropped for generated states.

## Where this happens
- `checkito/src/state.rs`
  - `State::random` uses `Sizes::from_ratio`.
  - `Sizes::from_ratio` returns `Self::new(..., Self::SCALE)` in both branches (`count <= 1` and `count > 1`).
- The same file also exposes APIs that preserve/customize scale, e.g. `With::size` and `With::scale`, which suggests scale is expected to be meaningful and configurable.

## Why this is an issue
1. **Configuration inconsistency**: `Sizes` carries `scale` as part of its state, but `from_ratio` effectively discards it.
2. **Behavior surprise**: A caller can set a custom scale and still get default-scaling behavior at runtime.
3. **Shrinking/generation distribution drift**: integer and float generation paths use `state.scale()`, so resetting it modifies sample distribution and can change failure discovery behavior.
4. **Hard-to-debug reproducibility differences**: users may think they are rerunning with custom scale settings when the runtime is actually using default scale.

## Reproduction outline
1. Construct a checker with a custom sizes scale (`checker.generate.sizes = checker.generate.sizes.scale(...)` via state plumbing or equivalent API path).
2. Generate several states and inspect `state.scale()` used by numeric generation.
3. Observe that generated states use default `Sizes::SCALE` rather than the configured scale.

## Proposed fix plan
1. Update `Sizes::from_ratio` to preserve the incoming `size.scale()` instead of `Self::SCALE`.
2. Add unit tests in `checkito/src/state.rs` (or nearby existing test module) for both branches:
   - `count <= 1` preserves scale.
   - `count > 1` preserves scale.
3. Add a behavior-level test proving custom scale actually affects generated numeric ranges (regression guard).
4. Confirm existing generation/shrinking tests still pass.

## Risk / compatibility notes
- This is likely a bug fix, but it can change observed sample distribution for callers who already provide custom scales and were unknowingly ignored.
- Document behavior in changelog/release notes as a correctness fix.
