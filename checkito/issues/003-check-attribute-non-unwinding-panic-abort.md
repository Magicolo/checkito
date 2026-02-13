# Issue: `#[check]` panic path can abort test binaries with "thread caused non-unwinding panic"

## Summary
Running `cargo test --test check` currently aborts the process (SIGABRT) on the `panics_with_option_unwrap` test with:

> thread caused non-unwinding panic. aborting.

This indicates a panic-handling path in `#[check]` execution is not robust for panicking properties.

## Evidence observed
Command:
- `cargo test --test check -- --nocapture --test-threads=1`

Observed sequence:
- Most tests pass.
- `panics_with_option_unwrap - should panic` triggers a hard abort rather than a normal unwind/catch/`should_panic` handling.

## Where this is likely rooted
- `checkito/src/run.rs`
  - Custom panic-hook management in `mod hook` (`begin`, `silent`, `end`, `panic`).
  - `hook::silent` temporarily removes thread-local hook state before running user check closure.
- `checkito/src/check.rs`
  - property function panic is intentionally wrapped with `catch_unwind`, converted to `Cause::Panic`.

There is likely an interaction bug between:
1. catch/unwind conversion in checker,
2. custom hook swapping/restoration,
3. explicit `hook::panic()` used after final fail reporting.

## Why this is an issue
- Breaks reliability of the check test harness.
- Causes abrupt process abort in normal expected panic scenarios.
- Can hide real failure diagnostics and disrupt CI (entire test binary aborts).

## Reproduction details
- Repro target already exists in tree: `panics_with_option_unwrap` in `checkito/tests/check.rs`.
- Repro command:
  - `cargo test --test check -- --nocapture --test-threads=1`

## Suggested fix plan
1. Add a focused regression test asserting panicing properties under `#[check]` are handled as normal property failures and compose with `#[should_panic]` without process abort.
2. Instrument panic-hook lifecycle (temporarily in test or debug assertions):
   - hook present before check,
   - restored after success/failure/panic paths.
3. Audit `hook::silent` for panic safety:
   - if user closure panics, ensure hook state restoration still occurs (RAII guard pattern may be needed).
4. Audit `hook::panic()` behavior to ensure it does not trigger a second panic in a non-unwind-safe context.
5. Re-run `cargo test --test check` and full test suite.

## Risk/impact
- High severity for maintainability and CI stability.
- Behavior directly affects core property-failure handling path.
