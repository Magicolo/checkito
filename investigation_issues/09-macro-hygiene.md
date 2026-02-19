# Macro Hygiene Issues in checkito_macro Crate

## Summary
The procedural macros in `checkito_macro` have several hygiene issues that can cause name collisions, confusing error messages, and potential bugs when used in certain contexts.

## Context
Macro hygiene ensures that:
1. Generated code doesn't accidentally capture user variables
2. Generated identifiers don't collide with user code
3. Error messages point to correct source locations
4. Macro expansion is predictable and safe

## Issues Found

### 1. **CRITICAL**: Function Name Collision (lib.rs:57-58)
**Location**: `checkito_macro/src/lib.rs:57-58`

**Current Code**:
```rust
let name = replace(&mut function.sig.ident, format_ident!("check"));
```

**Problem**:
- Replaces original function name with hardcoded `"check"`
- If multiple `#[check]` functions exist in same scope, **all become named `check()`**
- Causes compilation error due to duplicate definitions

**Example of Bug**:
```rust
#[check(0..10)]
fn test_positive(x: i32) { assert!(x >= 0); }

#[check(0..10)]
fn test_small(x: i32) { assert!(x < 100); }

// Both expand to:
// fn check() { ... }
// ERROR: duplicate definitions of 'check'
```

**Fix**: Use a unique, hygiene-preserving name:
```rust
let name = replace(&mut function.sig.ident, 
    format_ident!("__checkito_{}_{}", function.sig.ident, unique_id()));
```

Or use `quote_spanned!` with mixed site hygiene.

### 2. **HIGH**: Unhygienic Variable Names (check.rs:237)
**Location**: `checkito_macro/src/check.rs:237`

**Current Code**:
```rust
arguments.push(format_ident!("_{}", arguments.len()));
```

**Problem**:
- Generates identifiers `_0`, `_1`, `_2`, ...
- High collision risk if user code uses these names in closure

**Example of Bug**:
```rust
#[check(0..10, 0..10)]
fn test(a: i32, b: i32) {
    let _0 = "user variable";  // Collision with generated _0!
    assert!(a + b < 20);
}

// Expands to something like:
// let _0 = gen0.generate(&mut state);
// let _1 = gen1.generate(&mut state);
// closure(|_0, _1| { let _0 = "user variable"; ... })
```

**Fix**: Use hygiene-preserving prefixes:
```rust
arguments.push(format_ident!("__checkito_arg_{}", arguments.len()));
```

### 3. **MEDIUM**: Span Defaults to call_site (check.rs:196)
**Location**: `checkito_macro/src/check.rs:196`

**Current Code**:
```rust
None => (usize::MAX, usize::MAX, Span::call_site()),
```

**Problem**:
- When `rest` is `None`, uses `Span::call_site()` with dummy values
- Error messages may point to wrong location (macro definition instead of usage site)
- Confusing diagnostics for users

**Fix**: Use the actual span from the original syntax:
```rust
None => (usize::MAX, usize::MAX, self.span),
```

### 4. **MEDIUM**: Unwrap in Generated Code (regex.rs:47)
**Location**: `checkito_macro/src/regex.rs:47`

**Current Code**:
```rust
quote!(::checkito::regex(#pattern, #repeats).unwrap()).into()
```

**Problem**:
- Generates code with `.unwrap()` that panics if regex compilation fails
- While regex is validated at macro time (line 33), the generated code still has runtime risk
- Better to use compile-time validation or `expect` with better message

**Fix**: Use `expect` with descriptive message:
```rust
quote!(::checkito::regex(#pattern, #repeats)
    .expect("regex validated at compile time")).into()
```

Or better, inline the validated regex structure.

### 5. **LOW**: No Error Aggregation (check.rs:70-73)
**Location**: `checkito_macro/src/check.rs:70-73`

**Current Code**:
```rust
for check in checks {
    match check.run(&function.sig) {
        Ok(run) => runs.push(run),
        Err(error) => return error.to_compile_error().into(),
    }
}
```

**Problem**:
- Stops at first error, doesn't collect all errors
- Users get partial feedback, must fix errors one at a time
- Poor user experience for multi-attribute checks

**Fix**: Collect all errors:
```rust
let mut errors = Vec::new();
for check in checks {
    match check.run(&function.sig) {
        Ok(run) => runs.push(run),
        Err(error) => errors.push(error),
    }
}

if !errors.is_empty() {
    let combined = errors.into_iter()
        .map(|e| e.to_compile_error())
        .collect::<TokenStream>();
    return combined.into();
}
```

## Testing Strategy

### Test Hygiene Issues
```rust
// Test file: tests/macro_hygiene.rs

#[test]
fn multiple_checks_dont_collide() {
    // Should compile without errors
    #[check(0..10)]
    fn first(x: i32) { assert!(x >= 0); }
    
    #[check(0..10)]
    fn second(x: i32) { assert!(x < 100); }
}

#[test]
fn user_variables_dont_conflict() {
    #[check(0..10, 0..10)]
    fn test(a: i32, b: i32) {
        let _0 = 42;  // Should not conflict with generated _0
        let _1 = 99;  // Should not conflict with generated _1
        assert!(a + b + _0 + _1 < 200);
    }
}
```

### Test Error Messages
Verify error messages point to correct source locations.

## Priority

**High** - Hygiene issues can cause:
1. **Compilation failures** (name collisions)
2. **Confusing error messages** (wrong spans)
3. **Subtle bugs** (variable capture)

## Related Standards
- Rust RFC 1566: Procedural Macros
- Rust Reference: Macro Hygiene

## Estimated Effort
**Medium** - Requires:
1. Update identifier generation logic
2. Use proper span tracking
3. Add hygiene tests
4. Review all generated identifiers

## Acceptance Criteria
- [ ] No hardcoded "check" function name
- [ ] Generated variables use unique prefixes
- [ ] Error spans point to usage sites
- [ ] Multiple `#[check]` attributes work in same scope
- [ ] User variables don't conflict with generated code
- [ ] Error aggregation shows all issues at once
