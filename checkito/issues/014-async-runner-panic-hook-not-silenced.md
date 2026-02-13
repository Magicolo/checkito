# Issue: Asynchronous runner does not suppress panic hook output during property function execution

## Summary
In `checkito/src/run.rs`, the synchronous runner wraps property execution with `hook::silent(check)` to temporarily disable custom panic hook forwarding while executing user checks. The asynchronous runner currently cannot do this and contains a TODO:

```rust
// TODO: Is it possible to use `hook::silent` (adapted for futures) here?
```

As a result, async checks that panic can emit different/noisier output behavior than sync checks and may interact poorly with panic reporting consistency.

## Why this matters
- **Behavior inconsistency** between synchronous and asynchronous check modes.
- **Potentially noisy panic output** in async mode, especially in expected-failure scenarios.
- **Harder diagnostics** when comparing output across run modes or debugging CI failures.

## Relevant code context
- Synchronous path (`run::synchronous::with`):
  - `checker.checks(hook::silent(check))`
- Asynchronous path (`run::asynchronous::with`):
  - `checker.checks(check)` with TODO comment about hook adaptation.

## Risk profile
This is primarily an observability/consistency issue, but panic-hook handling is sensitive global state and can escalate to flaky behavior if not managed carefully.

## Investigation plan
1. Add focused async tests that intentionally panic and capture stderr/output.
2. Compare sync vs async output semantics for equivalent failing checks.
3. Implement an async-compatible `silent` wrapper strategy, such as:
   - disabling/restoring hook around poll lifecycle,
   - or a scoped guard integrated in async check dispatch path.
4. Validate that hook restoration is always balanced, including cancellation/drop paths.
5. Re-run existing panic-related tests (`check` integration tests and async tests).

## Implementation cautions
- Avoid introducing data races around global panic hook manipulation.
- Ensure nested/parallel async checks do not leak hook state across tasks.
- Document why the chosen strategy is safe with futures polling semantics.

## Acceptance criteria
- Async runner has equivalent panic-output suppression guarantees as sync runner (or explicit documented difference).
- Added regression tests protect against panic-hook output regressions.
- No new hangs/aborts in panic-heavy test scenarios.
