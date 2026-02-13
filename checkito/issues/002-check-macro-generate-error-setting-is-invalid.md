# Issue: `#[check]` macro accepts `generate.error` but runtime config has no such field

## Summary
The `checkito_macro` parser and code generator currently recognize the key `generate.error`, and emit code assigning `_checker.generate.error = ...`. However, the runtime `Generates` struct in `checkito/src/check.rs` does not define an `error` field.

This creates a latent API inconsistency where the macro advertises a configuration key that cannot compile if used.

## Why this is a problem
- Produces confusing compile errors for users when they use a seemingly documented/accepted key.
- Macro-level key validation should reject unsupported keys early and clearly.
- Indicates drift between proc-macro configuration surface and runtime checker API.

## Evidence and context
- `checkito_macro/src/check.rs`:
  - `Key::GenerateError` exists.
  - String mapping returns `"generate.error"`.
  - Update emission contains assignment to `_checker.generate.error`.
- `checkito/src/check.rs`:
  - `pub struct Generates` fields: `seed`, `sizes`, `count`, `items`, `exhaustive`.
  - No `error` field exists.

## Scope
- Primary file: `checkito_macro/src/check.rs`.
- Possibly affected docs/tests around accepted `#[check(...)]` settings.

## Proposed fix plan
1. **Write tests first:**
   - Add macro tests ensuring unsupported keys are rejected with a clear diagnostic.
   - Add a specific regression test for `generate.error`.
2. **Resolve API mismatch:** choose one of:
   - **Preferred:** remove `GenerateError` support from macro parsing and key listing.
   - **Alternative:** add an intentional `error` field and semantics in runtime `Generates` if truly desired.
3. **Update user-facing docs/messages:**
   - Ensure key list in errors includes only valid keys.
4. **Run tests:**
   - macro crate tests + integration tests for `#[check]`.

## Risks and caveats
- Removing a previously accepted key is technically a breaking change for any user relying on it (though currently broken/useless). Consider changelog note.
- If the key was intended for upcoming behavior, removal should include TODO or tracking issue for future feature.

## Acceptance criteria
- `generate.error` is either fully implemented end-to-end or clearly rejected by macro diagnostics.
- No generated code references missing checker fields.
- Key validation tests cover this mismatch.
