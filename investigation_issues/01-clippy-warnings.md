# Fix Clippy Warnings in checkito Library

## Summary
The checkito library currently has 8 clippy warnings that should be fixed to improve code quality, maintainability, and follow Rust best practices.

## Context
Running `cargo clippy --all-targets --all-features` reveals several warnings across the codebase. These warnings, while not blocking compilation, indicate areas where the code could be improved.

## Current Warnings

### 1. Unit Test in Doctest (README.md)
**Location**: `checkito/src/../../README.md:181`
**Warning**: `clippy::test_attr_in_doctest`
```
warning: unit tests in doctest are not executed
   --> checkito/src/../../README.md:181:1
    |
181 | #[test]
    | ^^^^^^^
```
**Issue**: The `#[test]` attribute in doctests doesn't execute. This can mislead users who copy code from documentation.

### 2. Large Error Variant in Result (check.rs)
**Locations**: 
- `checkito/src/check.rs:310` (`into_pass`)
- `checkito/src/check.rs:326` (`into_fail`)

**Warning**: `clippy::result_large_err`
```rust
pub fn into_pass(self, shrink: bool) -> result::Result<Pass<T, P::Proof>, Self>
pub fn into_fail(self, shrunk: bool) -> result::Result<Fail<T, P::Error>, Self>
```
**Issue**: The `Err` variant contains at least 136 bytes. Large error types can impact performance and stack usage.

**Recommendation**: Box the large error variant or refactor the type structure.

### 3. Redundant Closure (check.rs:530)
**Location**: `checkito/src/check.rs:530`
**Warning**: `clippy::redundant_closure`
```rust
catch_unwind(AssertUnwindSafe(move || run())).map_err(|error| Cause::Panic(cast(error).ok()))
```
**Recommendation**: Replace `move || run()` with just `run`.

### 4. Derivable Default Implementations (check.rs)
**Locations**:
- `checkito/src/check.rs:577-581` (Machine<G, P>)
- `checkito/src/check.rs:799-803` (Entry<S, P>)
- `checkito/src/check.rs:818-822` (Machine<G, P> async)

**Warning**: `clippy::derivable_impls`
```rust
impl<G: Generate, P: Prove> Default for Machine<G, P> {
    fn default() -> Self {
        Self::Done
    }
}
```
**Recommendation**: Use `#[derive(Default)]` and mark `Done` variant with `#[default]`.

### 5. Needless Lifetime (check.rs:1012)
**Location**: `checkito/src/check.rs:1012-1015`
**Warning**: `clippy::needless_lifetimes`
```rust
fn get<'a, S: Shrink, P: Future<Output: Prove>>(
    entries: Pin<&'a mut Box<[Entry<S, P>]>>,
    index: usize,
) -> Pin<&'a mut Entry<S, P>>
```
**Recommendation**: Elide the `'a` lifetime as it can be inferred.

## Plan for Fixing

1. **README doctest** (Line 181):
   - Remove `#[test]` attribute from doctest or convert to a regular test
   - Document that doctests don't run as unit tests

2. **Large Error Variant** (Lines 310, 326):
   - Consider boxing the error variant: `Result<Pass<T, P::Proof>, Box<Self>>`
   - Or restructure the enum to reduce size
   - Add `#[allow(clippy::result_large_err)]` with justification if intentional

3. **Redundant Closure** (Line 530):
   - Change `move || run()` to `run`

4. **Derivable Impls** (Lines 577, 799, 818):
   - Replace manual `Default` implementations with `#[derive(Default)]`
   - Mark default variant with `#[default]` attribute

5. **Needless Lifetime** (Line 1012):
   - Remove explicit `'a` lifetime annotations and let compiler infer

## Testing
- Run `cargo clippy --all-targets --all-features` to verify all warnings are fixed
- Run `cargo test` to ensure no regressions
- Run `cargo doc` to verify documentation still builds correctly

## Priority
Medium - These are code quality improvements that don't affect functionality but improve maintainability.

## Related Issues
- Issue #13: Fix cargo doc warnings (some overlap in documentation quality)
