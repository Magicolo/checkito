# Issue: asynchronous runner swaps `color` and `verbose` arguments

## Summary
The asynchronous execution path in `checkito/src/run.rs` appears to pass runtime display options in a different parameter order than the synchronous path. The private `with` function inside `run::asynchronous` takes arguments in `(verbose, color)` order, but all public entry points (`default`, `debug`, `minimal`) pass `(color, verbose)`.

This can cause option inversion at runtime (e.g., `verbose = true` can be interpreted as `color = true` and vice versa), producing surprising CLI output behavior and inconsistent semantics between sync and async checks.

## Why this is a problem
- User-facing behavior is inconsistent between synchronous and asynchronous check runs.
- Tests and docs implicitly assume these options are semantically stable across run modes.
- Output controls (`verbose`, `color`) are critical for debugging failing properties and CI readability.

## Evidence and context
- In `run::synchronous`, the helper signature is `with(..., color, verbose, ...)` and callers pass `(color, verbose)`.
- In `run::asynchronous`, callers pass `(color, verbose)`, but helper signature currently expects `(..., verbose, color, ...)`.
- The helper then forwards to `prepare(..., verbose, color)`, meaning values are likely flipped before output configuration is created.

## Scope
- Primary file: `checkito/src/run.rs`.
- Potentially affected tests: check-macro integration tests that exercise verbose/color attributes under async mode (if any).

## Proposed fix plan
1. **Add/adjust tests first (TDD):**
   - Add focused async tests that set `color` and `verbose` independently and assert behavior from emitted output formatting path (or internal state where accessible).
   - Ensure one test explicitly verifies that `verbose=true,color=false` does not emit ANSI color escapes.
2. **Fix helper signature/order:**
   - Normalize asynchronous helper signature to `(color, verbose)` to match callers and synchronous path.
3. **Audit call chain:**
   - Confirm argument order from macro-generated invocations into runtime API is consistent.
4. **Regression coverage:**
   - Add a small table-style test matrix for combinations of `(color, verbose)` in async mode.
5. **Run full relevant test suites:**
   - `cargo test --features check,asynchronous` (or project-standard equivalent).

## Risks and caveats
- If existing tests accidentally depended on swapped behavior, this change will break them. Those tests should be updated as they represent buggy baseline behavior.
- Output assertions should be robust to non-deterministic panic message text; prefer explicit marker checks.

## Acceptance criteria
- Async and sync modes treat `color` and `verbose` identically for equivalent settings.
- Added tests fail before the fix and pass after.
- No regressions in existing check macro tests.
