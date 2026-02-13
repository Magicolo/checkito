# Issue: `run::asynchronous::with` receives `color`/`verbose` in reversed order

## Summary
`checkito::run::asynchronous::{default,debug,minimal}` forward arguments to `with(...)` in the order `(color, verbose)`, but `with` currently declares those parameters as `(verbose, color)`. This means the values are silently swapped before they are passed into `prepare(...)`.

## Where this is in code
- `checkito/src/run.rs`
  - Call sites in `asynchronous::default`, `asynchronous::debug`, `asynchronous::minimal` pass:
    - `with(generator, update, check, color, verbose, handle_...)`
  - Callee signature is:
    - `fn with(..., verbose: bool, color: bool, handle: H)`
  - Callee then does:
    - `prepare(&mut checker, update, verbose, color)`

## Why this is an issue
This is a semantic correctness bug in user-facing behavior:
- `#[check(color = false)]` can unintentionally toggle `verbose`.
- `#[check(verbose = true)]` can unintentionally toggle `color`.
- Environment updates and explicit options become confusing because output configuration does not match user intent.

This is particularly risky because both fields are booleans and the compiler cannot catch accidental reordering.

## Reproduction direction
1. Add/adjust an async check test using `#[check(asynchronous = true, color = false, verbose = true)]`.
2. Assert that:
   - generated output contains verbose lines,
   - output contains no ANSI color escapes.
3. Current behavior should show the inverse in at least one of these controls.

## Suggested fix plan
1. Normalize argument order so all code paths use the same convention (recommended: `color, verbose` to match the public run API call style).
2. Update `asynchronous::with` signature and internal call to `prepare` accordingly.
3. Add targeted regression tests in `checkito/tests/asynchronous.rs` (or a new dedicated test file) that verify independent control of `color` and `verbose`.
4. Add a short inline comment in `run.rs` noting the canonical argument order to prevent regressions.

## Risk/impact
- Low implementation risk.
- High user-facing correctness impact for async checks.
- Good candidate for a quick fix with clear tests.
