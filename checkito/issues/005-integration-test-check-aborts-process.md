# Issue: `cargo test --test check` aborts with non-unwinding panic (SIGABRT)

## Summary
Running the `check` integration test target currently aborts the test process with:

- `thread caused non-unwinding panic. aborting.`
- `signal: 6, SIGABRT`

This indicates at least one panic path in the test/runtime interaction is aborting instead of unwinding, preventing full test completion.

## Why this is a problem
- Breaks CI stability and local developer confidence in test suite reliability.
- Masks true pass/fail status of remaining tests.
- Suggests panic-handling integration issue in runtime hooks, proc-macro-generated tests, or should-panic interactions.

## Reproduction
From repository root:

```bash
cargo test --test check -- --nocapture
```

Observed behavior:
- Most tests in `checkito/tests/check.rs` run and print expected pass/fail lines.
- Process aborts at end with non-unwinding panic.

## Evidence and context
- Failure is reproducible without code changes.
- The failure appears after tests that intentionally panic (`#[should_panic]`), pointing to panic handling + hook interaction as likely subsystem.
- `run.rs` contains custom panic hook management and async TODOs around hook silencing.

## Scope
- Test file: `checkito/tests/check.rs`.
- Runtime files likely involved: `checkito/src/run.rs`, possibly `checkito/src/check.rs` (panic capture paths).

## Investigation plan
1. **Isolate failing test(s):**
   - Run with `-- --nocapture --test-threads=1`.
   - Binary-search tests via `cargo test --test check <name>`.
2. **Enable backtraces and panic diagnostics:**
   - `RUST_BACKTRACE=1` and optionally `RUST_BACKTRACE=full`.
3. **Inspect panic strategy boundaries:**
   - Confirm no `panic = "abort"` config unexpectedly applied to test target.
   - Check for panic across FFI/non-unwind-safe boundaries.
4. **Correlate with hook lifecycle:**
   - Add temporary instrumentation in `hook::begin/end` to verify balanced entry/exit.
5. **Add regression test once root cause is known.**

## Potential root-cause directions
- Re-entrant panic hook behavior interacting with `hook::panic()` or nested checks.
- Panic during `Drop` while unwinding, escalating to abort.
- Interaction between macro-generated test wrappers and custom runner output handling.

## Acceptance criteria
- `cargo test --test check` exits cleanly with deterministic results.
- Root cause is documented and covered by regression test.
- Panic output and `#[should_panic]` semantics remain correct.
