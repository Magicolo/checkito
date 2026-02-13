# Issue: `#[check(generate.error = ...)]` key is parsed but expands to a non-existent checker field

## Summary
The `check` proc-macro accepts a `generate.error` key, but code generation maps it to `_checker.generate.error = ...`, and `Generates` has no `error` field. This produces compile-time expansion failures when users try to use the setting.

## Where this is in code
- `checkito_macro/src/check.rs`
  - `Key::GenerateError` exists in the enum and key list.
  - Key string mapping includes `"generate.error"`.
  - Update code emits `_checker.generate.error = #right;`.
- `checkito/src/check.rs`
  - `struct Generates` does not define an `error` field.

## Why this is an issue
- The macro surface advertises a configuration that is not actually supported.
- This creates a poor UX: users get confusing compiler errors coming from expanded code.
- It is also an API consistency problem: accepted syntax should correspond to valid runtime configuration.

## Reproduction direction
1. Add a minimal check test:
   - `#[check(generate.error = false)] fn x() {}`
2. Compile the test target.
3. Observe expansion error because `_checker.generate.error` does not exist.

## Suggested fix plan
Choose one path and make it explicit:

### Option A (recommended): Remove unsupported key
1. Remove `GenerateError` from `Key` and `KEYS`.
2. Remove string conversion and match arms for it.
3. Add compile-time diagnostics test to assert key is rejected with a clear message listing supported keys.
4. Update docs/examples to avoid mentioning this key (if present).

### Option B: Implement actual setting
1. Define a real field on `Generates` and behavior where it is consumed.
2. Ensure checker logic uses the field in a meaningful way.
3. Add end-to-end tests validating semantics.

Given current code, Option A is likely correct unless there is unfinished design work elsewhere.

## Risk/impact
- Medium impact on users relying on macro settings.
- Low code-change risk for Option A.
- Should be fixed before adding more macro settings to avoid API drift.
