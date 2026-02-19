# Missing Documentation for check::Result Methods

## Summary
The `check::Result` enum and its accessor methods in `checkito/src/check.rs` (lines 302-347) lack documentation, making it difficult for users to understand how to work with check results programmatically.

## Context
The `check::Result` enum is a core type returned by the checking process with four variants:
- `Pass(Pass<T, P::Proof>)` - Property held
- `Shrink(Pass<T, P::Proof>)` - Property held after shrinking
- `Shrunk(Fail<T, P::Error>)` - Property failed after shrinking
- `Fail(Fail<T, P::Error>)` - Property failed

Users who want to process check results programmatically (e.g., in custom test harnesses or CI integrations) need clear documentation of the accessor methods.

## Missing Documentation

### Undocumented Methods (Lines 302-347)

#### 1. `Result::pass(&self, shrink: bool) -> Option<&Pass<T, P::Proof>>` (Line 302)
**Current**: No documentation
**Needed**:
```rust
/// Returns a reference to the passing result if this is a `Pass` or `Shrink` variant.
///
/// # Arguments
/// * `shrink` - If `true`, includes `Shrink` variant. If `false`, only matches `Pass`.
///
/// # Examples
/// ```rust
/// use checkito::*;
///
/// let result = (0..10).check(|x| x < 100);
/// if let Some(pass) = result.pass(false) {
///     println!("Test passed with value: {:?}", pass.item);
/// }
/// ```
```

#### 2. `Result::into_pass(self, shrink: bool) -> result::Result<Pass<T, P::Proof>, Self>` (Line 310)
**Current**: No documentation
**Needed**: Explain consuming variant with example

#### 3. `Result::fail(&self, shrunk: bool) -> Option<&Fail<T, P::Error>>` (Line 318)
**Current**: No documentation
**Issue**: The parameter name `shrunk` is confusing - it includes both `Fail` and `Shrunk` variants when `true`

#### 4. `Result::into_fail(self, shrunk: bool) -> result::Result<Fail<T, P::Error>, Self>` (Line 326)
**Current**: No documentation

#### 5. `Result::into_item(self) -> T` (Line 334)
**Current**: No documentation
**Needed**: Clarify that this extracts the generated item regardless of pass/fail

#### 6. `Result::into_result(self) -> result::Result<Pass<T, P::Proof>, Fail<T, P::Error>>` (Line 342)
**Current**: No documentation
**Needed**: Explain how `Shrink` and `Shrunk` variants map to `Ok`/`Err`

### Minimal Documentation (Lines 369)

#### `Fail::message(&self) -> String` (Line 369)
**Current**: Only has a single-line comment
**Needed**: Full documentation with examples showing different error types

## Additional Issues

### 1. Confusing Method Naming
The `pass(shrink: bool)` and `fail(shrunk: bool)` methods have counter-intuitive boolean parameters:
- `pass(true)` means "include Shrink variant" (not "exclude Pass variant")
- `fail(true)` means "include Shrunk variant" (not "exclude Fail variant")

This is opposite to Rust's standard `Result::ok()/err()` which DON'T have parameters.

**Recommendation**: Consider separate methods:
```rust
fn pass_only(&self) -> Option<&Pass<T, P::Proof>>
fn pass_or_shrink(&self) -> Option<&Pass<T, P::Proof>>
fn fail_only(&self) -> Option<&Fail<T, P::Error>>
fn fail_or_shrunk(&self) -> Option<&Fail<T, P::Error>>
```

### 2. No Guidance on Result Processing
Users need examples showing:
- How to extract Pass from successful checks
- How to handle Shrunk vs Fail differently
- How to build custom result processors
- How to integrate with CI/test frameworks

## Recommended Documentation Structure

### For check.rs Module (Lines 177-192)
Add module-level example:
```rust
/// # Working with Check Results
///
/// ```rust
/// use checkito::*;
///
/// for result in (0..100).checks(|x| x < 50) {
///     match result {
///         check::Result::Pass(pass) => println!("✓ Passed: {:?}", pass.item),
///         check::Result::Shrink(pass) => println!("✓ Shrunk: {:?}", pass.item),
///         check::Result::Shrunk(fail) => println!("✗ Failed (shrunk): {:?}", fail.item),
///         check::Result::Fail(fail) => println!("✗ Failed: {:?}", fail.item),
///     }
/// }
/// ```
```

### For Each Accessor Method
Add:
1. One-line summary
2. Parameter descriptions
3. Return value semantics
4. At least one code example
5. Links to related methods

## Impact on Users
**High** - These methods are part of the public API, and users who want to:
- Build custom test harnesses
- Integrate with CI systems
- Process results programmatically
- Generate custom reports

...all need clear documentation to use these methods effectively.

## Related Issues
- Issue #8: "Add doc examples/tests in main traits" - This is specifically for `check::Result` accessor methods
- Issue #13: "Fix cargo doc warnings" - Some overlap in documentation quality

## Priority
**High** - Core public API lacking documentation

## Acceptance Criteria
- [ ] All `check::Result` methods have rustdoc comments
- [ ] Each method has at least one working example
- [ ] Module-level example shows result processing
- [ ] `cargo doc` generates useful API documentation
- [ ] Examples compile and run correctly
