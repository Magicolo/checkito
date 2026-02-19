# Missing Documentation for Macro Attributes in #[check]

## Summary
The `#[check]` procedural macro in `checkito_macro` supports many attributes (`generate.count`, `shrink.count`, `debug`, `color`, `verbose`, `constant`, `asynchronous`, etc.) but these are severely under-documented or not documented at all.

## Context
Users need to understand all available configuration options for `#[check]` to effectively use the library. Currently, documentation is scattered or missing entirely.

## Current Documentation State

**Location**: `checkito_macro/src/lib.rs:39-43`

**Current**:
```rust
/// Turns a function into a property test.
///
/// Accept parameters that modify the test behavior and a list of expressions that
/// implement `Generate`. The expressions are used to generate values to feed to the
/// function as arguments.
```

**Problem**: Only mentions "parameters" and "Generate expressions" without listing them or explaining their usage.

## Missing Documentation

### 1. Generate Configuration Attributes
**No documentation for**:
- `generate.count` - Number of test cases to run
- `generate.seed` - Random seed for reproducibility
- `generate.sizes` - Size range for generated values
- `generate.items` - Whether to display passing items
- `generate.exhaustive` - Enable exhaustive testing mode

**Should document**:
```rust
/// # Generate Configuration
/// 
/// - `generate.count = N` - Run N test cases (default: 100)
/// - `generate.seed = N` - Set random seed for reproducibility
/// - `generate.sizes = start..end` - Control size distribution (default: 0.0..=1.0)
/// - `generate.items = true/false` - Display each passing test item
/// - `generate.exhaustive = true/false` - Enable exhaustive testing when possible
///
/// # Examples
/// ```rust
/// #[check(0..100, generate.count = 1000, generate.seed = 42)]
/// fn test_with_config(x: i32) {
///     assert!(x < 100);
/// }
/// ```
```

### 2. Shrink Configuration Attributes
**No documentation for**:
- `shrink.count` - Maximum shrink attempts
- `shrink.items` - Whether to display passing shrink steps
- `shrink.errors` - Whether to display failing shrink steps

**Should document**:
```rust
/// # Shrink Configuration
///
/// - `shrink.count = N` - Maximum shrink attempts (default: 1000)
/// - `shrink.items = true/false` - Display each passing shrink step
/// - `shrink.errors = true/false` - Display each failing shrink step
///
/// # Examples
/// ```rust
/// #[check(0..1000, shrink.count = 500, shrink.items = true)]
/// fn test_with_shrink_config(x: i32) {
///     assert!(x < 100); // Will show shrinking steps
/// }
/// ```
```

### 3. Display and Output Attributes
**No documentation for**:
- `debug` - Require Debug trait on inputs (default: true)
- `color` - Enable colored output (default: true)
- `verbose` - Show detailed test execution

**Should document**:
```rust
/// # Display Configuration
///
/// - `debug = true/false` - Require `Debug` on generated values (default: true)
/// - `color = true/false` - Enable colored output (default: true)
/// - `verbose = true/false` - Show detailed execution information
///
/// # Examples
/// ```rust
/// // Test non-Debug types
/// #[check(my_generator(), debug = false)]
/// fn test_no_debug(val: MyType) { }
///
/// // Disable color for CI
/// #[check(0..100, color = false, verbose = true)]
/// fn test_no_color(x: i32) { }
/// ```
```

### 4. Special Mode Attributes
**No documentation for**:
- `constant` - Treat generator as constant (run once)
- `asynchronous` - Enable async execution

**Should document**:
```rust
/// # Special Modes
///
/// - `constant = true/false` - Treat generators as constants, run test once
/// - `asynchronous = true/false` - Enable async test execution
///
/// # Examples
/// ```rust
/// // Parameterized unit test (runs once)
/// #[check(42, 58, constant = true)]
/// fn test_constant(a: i32, b: i32) {
///     assert_eq!(a + b, 100);
/// }
///
/// // Async property test
/// #[check(0..100, asynchronous = true)]
/// async fn test_async(x: i32) {
///     let result = async_operation(x).await;
///     assert!(result.is_ok());
/// }
/// ```
```

### 5. Generator Syntax (`_` and `..` operators)
**No documentation for**:
- `_` - Infer default generator for type
- `..` - Variable-length generator arguments
- `.., gen` - Rest pattern with explicit last generator
- `gen1, .., gen2` - Rest pattern with explicit first and last

**Should document**:
```rust
/// # Generator Syntax
///
/// - `_` - Infer the default generator for the type
/// - `..` - Variable number of arguments, all using default generators
/// - `.., gen` - Variable arguments followed by explicit generator
/// - `gen1, .., gen2` - Explicit first and last, rest use defaults
///
/// # Examples
/// ```rust
/// // Infer all generators
/// #[check(_, _, _)]
/// fn test_infer(a: i32, b: String, c: bool) { }
///
/// // Variable number of arguments
/// #[check(..)]
/// fn test_variadic(a: i32, b: i32, c: i32) { }
///
/// // Mix explicit and inferred
/// #[check(0..100, .., letter())]
/// fn test_mixed(a: i32, b: i32, c: i32, d: char) { }
/// ```
```

### 6. Multiple #[check] Attributes
**No documentation explaining**:
- Multiple `#[check]` on same function runs multiple test scenarios
- Each attribute is independent
- Useful for parameterized testing

**Should document**:
```rust
/// # Multiple Checks
///
/// Multiple `#[check]` attributes run independent test scenarios:
///
/// ```rust
/// #[check(0, 10)]     // Tests with a=0, b=10
/// #[check(5, 5)]      // Tests with a=5, b=5
/// #[check(100, -100)] // Tests with a=100, b=-100
/// fn test_addition(a: i32, b: i32) {
///     assert_eq!(a + b, b + a); // Commutative property
/// }
/// ```
```

## Comparison with README

**README.md** (Lines 50-172) has **excellent examples** but they're not in rustdoc.

**Problem**: 
- Users reading API docs (`cargo doc`) won't see these examples
- Examples should be in both README and rustdoc
- Rustdoc is the canonical API reference

## Recommended Structure

**In `checkito_macro/src/lib.rs`**:
```rust
/// Turns a function into a property test.
///
/// The `#[check]` attribute runs your function multiple times with randomly
/// generated inputs to find failing test cases. When a failure is found,
/// it automatically shrinks the input to find the minimal failing case.
///
/// # Basic Usage
/// ```rust
/// use checkito::*;
///
/// #[check(0..100)]
/// fn test_less_than_100(x: i32) {
///     assert!(x < 100);
/// }
/// ```
///
/// # Generator Arguments
/// [... copy relevant examples from README ...]
///
/// # Configuration Attributes
/// [... document all generate.*, shrink.*, debug, color, verbose, etc. ...]
///
/// # Multiple Checks
/// [... document multiple #[check] usage ...]
///
/// # Async Support
/// [... document asynchronous mode ...]
///
/// # Environment Variables
/// [... document CHECKITO_* env vars ...]
///
/// See also: [`Check`], [`Checker`], [`Generate`]
```

## Impact

**High** - The `#[check]` macro is the primary entry point for most users. Without comprehensive documentation:
- Users don't know what features exist
- Users can't effectively configure tests
- Support burden increases (users ask same questions)
- Adoption is hindered

## Related Issues
- Issue #8: "Add doc examples/tests in main traits"
- Issue #7: "Add library documentation in lib.rs"
- README.md has examples that should be in rustdoc

## Priority
**High** - Critical for usability and adoption

## Acceptance Criteria
- [ ] All `#[check]` attributes documented
- [ ] Each attribute has at least one example
- [ ] Generator syntax (`_`, `..`) explained with examples
- [ ] Multiple `#[check]` usage documented
- [ ] Async mode documented
- [ ] Environment variables documented
- [ ] Cross-references to related traits/types
- [ ] Examples compile and run correctly
- [ ] `cargo doc` output is comprehensive and navigable
