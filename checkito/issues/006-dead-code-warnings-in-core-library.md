# Issue: dead-code warnings in core library (`Modes::{state_unchecked,count}`, `utility::cast_ref`)

## Summary
Current test/build output reports dead-code warnings for internal APIs:
- `Modes::state_unchecked`
- `Modes::count`
- `utility::cast_ref`

These warnings indicate either incomplete feature integration or stale code paths that should be removed/refactored.

## Why this is a problem
- Warnings reduce signal/noise and can hide more important warnings.
- Unused internal APIs increase maintenance surface and cognitive load.
- In this project, some unused functions appear tied to optional features (e.g., parallel/panic handling), suggesting architectural drift.

## Evidence
Observed during `cargo test` and `cargo test --test check`:
- `checkito/src/state.rs`: methods `state_unchecked` and `count` are never used.
- `checkito/src/utility.rs`: function `cast_ref` is never used.

## Scope
- `checkito/src/state.rs`
- `checkito/src/utility.rs`
- potentially `checkito/src/parallel.rs` if `cast_ref` was meant to be used there under feature gates.

## Proposed fix plan
1. **Audit intended usage:**
   - Determine whether each item is planned for near-term use (document with TODO + tests) or can be removed.
2. **If removal is correct:**
   - Delete dead code and update references/docs.
3. **If retention is needed:**
   - Wire into active codepaths and add tests proving usage.
   - Alternatively add precise `#[allow(dead_code)]` with rationale (last resort).
4. **Re-run full workspace tests and verify warning-free output.**

## Risks and caveats
- Removing currently-unused APIs may affect downstream users if they were `pub` and relied upon (these are `pub(crate)`/internal in current observations).
- If feature-gated usage exists, test all relevant feature combinations before deletion.

## Acceptance criteria
- No dead-code warnings for these symbols under standard test builds.
- Retained code has documented rationale and test coverage.
