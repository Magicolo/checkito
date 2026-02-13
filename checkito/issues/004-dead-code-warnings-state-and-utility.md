# Issue: persistent dead-code warnings indicate stale/unfinished API paths

## Summary
Current build emits dead-code warnings for:
- `Modes::state_unchecked`
- `Modes::count`
- `utility::cast_ref`

Warnings are not fatal, but they signal either stale code paths or missing integration.

## Evidence observed
Commands:
- `cargo test -q`
- `cargo clippy -q`

Warnings:
- `checkito/src/state.rs`: methods `state_unchecked` and `count` are never used.
- `checkito/src/utility.rs`: function `cast_ref` is never used.

## Why this is an issue
- In a correctness-sensitive generator library, stale paths can mask incomplete refactors.
- Extra dead code increases maintenance cost and cognitive load.
- Warnings hide newly introduced warnings in CI logs.

## Context clues
- `cast_ref` appears to be intended for panic payload extraction in parallel internals, but no active path currently references it in the default compiled library surface.
- `Modes::state_unchecked`/`count` appear utility-like but are not consumed by iterator construction or tests.

## Suggested fix plan
1. Decide for each symbol: keep or remove.
2. If keeping:
   - add real call sites and tests proving why the symbol exists.
   - document intended use (especially for unsafe-ish naming like `state_unchecked`).
3. If removing:
   - delete unused functions and simplify related code.
4. Add lint policy guidance for dead code in this crate (optional but recommended).

## Risk/impact
- Low immediate runtime risk.
- Medium long-term maintainability risk.
