# Issue: tuple-based `Any` generation duplicates weighted/indexed selection logic (missing `State::any_tuple*` helpers)

## Summary
`checkito/src/any.rs` includes a TODO to use `State::any_tuple`, and tuple generation currently implements selection logic inline. This duplicates choice behavior already conceptually similar to indexed/weighted selection for slices.

Centralizing tuple-selection in `State` would reduce duplication and improve consistency across `Any` combinators.

## Why this is a problem
- Duplicated selection code is harder to audit for correctness and probability semantics.
- Any future changes in weighting behavior require touching multiple locations.
- Increases risk of drift between tuple `Any` and slice/vector `Any` behavior.

## Evidence and context
- `checkito/src/any.rs` has TODO comment: `Use State::any_tuple`.
- `checkito/src/state.rs` has TODO comment: `Implement any_tuple_indexed and any_tuple_weighted`.
- Current tuple implementation includes local weighted-selection branches and unreachable assumptions.

## Scope
- `checkito/src/any.rs`
- `checkito/src/state.rs`
- tests for `Any` tuple/slice/weighted generation behavior

## Proposed fix plan
1. **Test-first:**
   - Add behavior-alignment tests ensuring tuple `Any` and equivalent slice/weighted forms produce compatible distribution/selection constraints.
2. **Add state helpers:**
   - Implement `State::any_tuple_indexed` and `State::any_tuple_weighted` with clear contracts.
3. **Refactor `any.rs` tuple macro paths:**
   - Replace local selection logic with new helpers.
4. **Document invariants:**
   - Why unreachable branches are safe (or remove unreachable by returning explicit fallbacks/errors if invariants can be violated).
5. **Run full generator/shrinker tests.**

## Risks and caveats
- Distribution differences may subtly change existing probabilistic expectations.
- Ensure cardinality calculations remain unchanged where expected.

## Acceptance criteria
- Tuple and slice-based `Any` share centralized selection logic.
- Existing behavior is preserved or intentionally documented where changed.
- New tests protect against regressions and invariant drift.
