# Issue: exhaustive numeric generation ignores size-biasing toward small values

## Summary
In `checkito/src/state.rs`, integer and floating-point generators apply size-aware shrinking for `Mode::Random`, but `Mode::Exhaustive` currently enumerates raw range-space via `consume(...)` without any equivalent small-value prioritization.

There are explicit TODO comments noting this gap:
- integer generator exhaustive branch
- floating generator exhaustive branch

## Why this is a problem
- Property testing usually benefits from exercising simpler/smaller values early.
- When auto-switching to exhaustive mode (based on cardinality), users may observe very different discovery behavior than random mode.
- This can reduce reproducibility intuition: identical generator config with only mode change can dramatically alter failure discovery order.

## Evidence and context
- `Mode::Random` path applies non-linear size adjustment (`shrink(...)`) for numeric ranges.
- `Mode::Exhaustive` path maps index directly to value space via `consume` and bit transforms.
- TODO comments directly suggest implementing small-first behavior for exhaustive as well.

## Scope
- `checkito/src/state.rs` numeric generation macros (`integer!`, `floating!`).
- Potentially test suites that rely on exact exhaustive ordering.

## Proposed fix plan
1. **Define ordering contract first:**
   - Decide whether exhaustive means complete coverage only, or complete coverage with deterministic *biased order*.
2. **Add tests before implementation:**
   - For representative ranges crossing zero, assert first N generated values are small/centered.
   - Ensure full coverage remains exact after complete iteration.
3. **Implement deterministic permutation/index mapping:**
   - Keep one-to-one mapping from exhaustive index space to range values.
   - Apply center-out or magnitude-tier ordering while preserving determinism.
4. **Document behavior:**
   - Explain why exhaustive ordering differs from linear ascending range order.
5. **Regression testing:**
   - Run state-specific tests + integration tests that may depend on ordering assumptions.

## Risks and caveats
- Changing exhaustive order may affect tests or users who implicitly relied on current enumeration order.
- Must preserve full cardinality and avoid duplicates/skips.

## Acceptance criteria
- Exhaustive mode remains complete and deterministic.
- Early exhaustive samples prioritize smaller-magnitude values in numeric ranges.
- Behavior is documented and covered by focused tests.
