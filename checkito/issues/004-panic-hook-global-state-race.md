# Issue: panic hook management in `run` uses global process state without concurrency guard

## Summary
`checkito/src/run.rs` temporarily overrides the global panic hook using `panic::set_hook`, while storing/restoring the previous hook in thread-local state. Because panic hooks are process-global, concurrent or nested check runs can interfere with each other.

The current implementation may still work in many cases, but it has race-prone semantics under parallel test execution.

## Why this is a problem
- Global hook replacement is inherently shared across threads.
- Two check runs entering/exiting around the same time can restore hooks in surprising order.
- Could lead to lost/misrouted panic formatting, flaky output, or hook leakage across tests.

## Evidence and context
- `hook::begin` takes and sets the global panic hook.
- `hook::end` restores whichever hook is in TLS for current thread.
- TLS storage does not serialize across threads; global hook writes still race.
- Synchronous path wraps checks with `hook::silent`, async path has TODO noting missing equivalent behavior.

## Scope
- Primary file: `checkito/src/run.rs` (`mod hook`).
- Indirectly affects all `#[check]` runtime behavior and panic output handling.

## Proposed fix plan
1. **Create targeted tests first:**
   - Nested `Guard` usage test.
   - Multi-thread test with concurrent begin/end cycles and assertion that original hook is restored.
2. **Introduce global synchronization:**
   - Use `Mutex` (or similar) around global hook lifecycle.
   - Add reference-counting for nested guard scopes.
3. **Preserve desired behavior:**
   - Keep panic output suppression behavior for check execution.
   - Restore original hook exactly once at outermost drop.
4. **Validate with async path as well:**
   - Ensure no regressions for `asynchronous` module usage.

## Risks and caveats
- Hook management is subtle and can itself become deadlock-prone if lock usage crosses panic boundaries. Keep critical section minimal.
- Tests should avoid depending on exact panic message formatting from Rust runtime.

## Acceptance criteria
- Original hook restoration is deterministic under nested and concurrent usage.
- No observed hook leakage between unrelated tests.
- Check output behavior remains stable in sync and async modes.
