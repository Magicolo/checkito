# Issue: exhaustive float generation order is not size-aware (known TODO)

## Summary
`State` has TODOs indicating exhaustive float generation does not currently prioritize smaller/simpler values first. This can degrade shrinking efficiency and predictability for floating-point properties.

## Where this is in code
- `checkito/src/state.rs`
  - TODO comments in exhaustive branches of float generation:
    - "Generate 'small' values first. Maybe use the same adjustment as Random?"

## Why this matters
Property testing generally benefits when generation starts with simpler values:
- Faster discovery of minimal failing cases.
- Better overlap with shrink strategy.
- More predictable edge-case progression.

Current exhaustive float generation maps indices directly over bit-space ordering, which is deterministic but not semantically "small-to-large" in numeric complexity.

## Potential symptoms
- Early exhaustive samples for float ranges may look unintuitive.
- More shrinking work needed to reach minimal counterexamples.
- Inconsistent experience compared with integer/string generators that often have clearer small-value bias.

## Suggested fix plan
1. Define an explicit exhaustive ordering policy for floats (e.g., `0`, small magnitudes, signs, then wider magnitudes).
2. Align exhaustive ordering with random-mode bias where practical.
3. Add focused tests that verify early sequence characteristics for representative ranges:
   - `-1.0..=1.0`
   - large symmetric ranges
   - subnormal/normal boundary scenarios
4. Ensure determinism remains stable across seeds/modes.
5. Document ordering guarantees and any intentional trade-offs.

## Risk/impact
- Medium implementation complexity.
- High value for test quality and shrink ergonomics.
