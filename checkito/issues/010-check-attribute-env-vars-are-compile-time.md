# Issue: `#[check]` env-based defaults (`CHECKITO_DEBUG/COLOR/VERBOSE/ASYNCHRONOUS`) are compile-time, not runtime

## Summary
The `checkito_macro` parser reads `CHECKITO_DEBUG`, `CHECKITO_COLOR`, `CHECKITO_VERBOSE`, and `CHECKITO_ASYNCHRONOUS` in `Check::new`. Because this logic is inside a proc-macro crate, these values are captured at macro expansion (compile) time.

This differs from runner configuration in `checkito/src/run.rs`, which reads generation/shrinking env vars at runtime.

## Where this happens
- `checkito_macro/src/check.rs`
  - `Check::new` initializes defaults via `parse("CHECKITO_DEBUG")`, `parse("CHECKITO_COLOR")`, `parse("CHECKITO_VERBOSE")`, and `parse("CHECKITO_ASYNCHRONOUS")`.
- `README.md`
  - Documents runtime environment overrides for generation/shrinking, but does not clearly document compile-time semantics for these macro defaults.

## Why this is an issue
1. **Unexpected semantics**: users generally expect `cargo test` env vars to affect current execution, not require recompilation.
2. **Incremental build confusion**: changing env var values without forcing recompilation may have no effect.
3. **Inconsistent model**: some env vars are runtime (`CHECKITO_GENERATE_*`, `CHECKITO_SHRINK_*`), others are compile-time (macro defaults).
4. **CI variability risk**: different build/test stages may inadvertently apply different defaults depending on when compilation occurred.

## Reproduction outline
1. Build tests with `CHECKITO_VERBOSE=false`.
2. Re-run tests with `CHECKITO_VERBOSE=true` **without** recompiling touched macro call sites.
3. Observe unchanged verbosity behavior from `#[check]` expansion defaults.
4. Force recompilation; behavior now updates.

## Proposed fix plan
Pick one explicit strategy and document it:

### Option A (preferred): move defaults to runtime
1. Stop reading these env vars inside proc-macro parsing.
2. Thread unresolved/default options into runtime runner logic (similar to existing runtime env handling).
3. Ensure `#[check(...)]` explicit attribute arguments still take precedence.

### Option B: keep compile-time behavior but make it explicit
1. Add clear docs that these specific env vars are compile-time macro defaults.
2. Add compile-test/docs test showing recompilation requirement.
3. Consider renaming vars or introducing runtime variants to reduce ambiguity.

## Test plan
- Add integration coverage demonstrating expected precedence:
  1. explicit attribute arg,
  2. env default,
  3. built-in default.
- If Option A is implemented, add runtime env tests in `checkito/tests` to ensure no recompilation dependency.
