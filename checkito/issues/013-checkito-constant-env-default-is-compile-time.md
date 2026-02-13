# Issue: `CHECKITO_CONSTANT` default is resolved at proc-macro expansion time (compile-time), not runtime

## Summary
When the `constant` feature is enabled, `checkito_macro::check::Check::new` reads `CHECKITO_CONSTANT` using `std::env` from inside the proc-macro crate. This means the environment variable is captured during compilation, not when tests execute.

This mirrors the known compile-time behavior of other macro defaults (`CHECKITO_DEBUG/COLOR/VERBOSE/ASYNCHRONOUS`), but `CHECKITO_CONSTANT` is currently not explicitly tracked in existing issue docs and is easy for users to misinterpret.

## Why this is an issue
- **Unexpected behavior**: toggling `CHECKITO_CONSTANT` between test runs may have no effect without recompilation.
- **Inconsistent mental model**: runtime env variables in `run.rs` (generation/shrinking) are read at execution time, while this one is compile-time.
- **Hard debugging**: users may think constant conversion is “flaky” when it is actually stale build artifact behavior.

## Where this happens
- Macro-side default initialization:
  - `checkito_macro/src/check.rs` in `Check::new`:
    - `constant: parse("CHECKITO_CONSTANT")` (behind `feature = "constant"`)
- Runtime env updates do **not** include a corresponding `CHECKITO_CONSTANT` concept:
  - `checkito/src/run.rs` environment module only handles `CHECKITO_GENERATE_*` and `CHECKITO_SHRINK_*`.

## Reproduction outline
1. Build tests with `CHECKITO_CONSTANT=false` (or unset) and run macro-annotated tests.
2. Re-run with `CHECKITO_CONSTANT=true` without forcing recompilation.
3. Observe behavior remains tied to compile-time-expanded value.
4. Force rebuild; behavior changes.

## Fix plan options
### Option A (preferred): make behavior explicit in docs
1. Document `CHECKITO_CONSTANT` as a **compile-time proc-macro default**.
2. Add examples clarifying that changing it requires recompilation.
3. Include it in the same documentation section as other compile-time macro env defaults.

### Option B: move decision to runtime
1. Stop binding `constant` default in proc-macro env parsing.
2. Thread an explicit runtime override mechanism (if design allows).
3. Keep explicit attribute argument precedence unchanged.

## Test/documentation plan
- Add integration test or compile-time test fixture proving compile-time capture semantics.
- Update README/docs where env vars are described, separating compile-time vs runtime categories.

## Acceptance criteria
- Behavior is clearly intentional (either runtime or compile-time), documented, and tested.
- Users can reliably predict how `CHECKITO_CONSTANT` affects `#[check]` invocations.
