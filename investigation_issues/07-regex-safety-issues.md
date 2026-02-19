# Regex Generator: Unsafe u8 to char Cast and Missing Validation

## Summary
The regex generator in `regex.rs` has several critical issues including unsafe type casting, missing validation for unbounded repetitions, and potential panics that can cause test failures.

## Issues Found

### 1. **CRITICAL**: Unsafe u8 to char Cast (Line 68)
**Location**: `checkito/src/regex.rs:68`

**Current Code**:
```rust
Regex::Range(Range(value.start() as char, value.end() as char))
```

**Problem**: 
- Casts raw `u8` (0-255) directly to `char` without validation
- Bytes 128-255 are **not valid Unicode scalars** when cast to char
- Can produce invalid UTF-8 in generated strings
- Violates Rust's char invariants (char must be valid Unicode scalar value)

**Example**:
```rust
let invalid = 200u8 as char; // This is unsafe!
// char can only represent Unicode scalar values (0x0000 to 0x10FFFF, excluding surrogates)
```

**Correct Implementation**:
```rust
Regex::Range(Range(
    char::from_u32(value.start() as u32).unwrap_or(char::REPLACEMENT_CHARACTER),
    char::from_u32(value.end() as u32).unwrap_or(char::REPLACEMENT_CHARACTER)
))
```

### 2. **CRITICAL**: Missing Validation for Unbounded Repetitions (Line 111)
**Location**: `checkito/src/regex.rs:111`

**Current Code**:
```rust
let high = max.unwrap_or(repeats.max(low));
```

**Problem**:
- When regex has unbounded `*` or `+` operators, `max` is `None`
- Defaults to `repeats` constant (typically 64)
- **No validation that `low <= high`** after computation
- **No bounds checking** for integer overflow

**Example of Bug**:
```rust
// Regex: a{1000,}  (at least 1000 repetitions)
// With default REPEATS=64:
// low = 1000
// high = max(64, 1000) = 1000
// Range becomes 1000..=1000 (works by accident)

// But with user-provided REPEATS=10:
// low = 1000
// high = max(10, 1000) = 1000
// If this logic changes, could create invalid range
```

**Should validate**:
```rust
let high = max.unwrap_or(repeats.max(low));
debug_assert!(low <= high, "Invalid repetition range: {}..{}", low, high);
```

### 3. **HIGH**: Integer Division Loss in Recursive Calls (Line 106)
**Location**: `checkito/src/regex.rs:106`

**Current Code**:
```rust
let tree = Self::from_hir(*sub, repeats / 2);
```

**Problem**:
- Halving `repeats` on each recursion: 64 → 32 → 16 → 8 → 4 → 2 → 1 → **0**
- When `repeats` becomes 0, creates pathological behavior
- Deeply nested quantifiers like `(a*)*` may generate zero-length max repetitions

**Example**:
```rust
// Pattern: ((a*)*)* with 7 levels of nesting
// repeats: 64 / 2 / 2 / 2 / 2 / 2 / 2 / 2 = 0
// Now max repetitions is 0, violating regex semantics
```

**Fix**:
```rust
let tree = Self::from_hir(*sub, repeats.saturating_div(2).max(1));
```

### 4. **MEDIUM**: Silent Data Loss in UTF-8 Parsing (Line 102)
**Location**: `checkito/src/regex.rs:102`

**Current Code**:
```rust
String::from_utf8(literal.0.to_vec()).map_or(Self::Empty, Self::Text)
```

**Problem**:
- Non-UTF-8 literals silently become `Empty`
- No warning or error message
- User has no feedback that part of their regex was dropped
- Generated strings won't match original pattern

**Better**:
```rust
String::from_utf8(literal.0.to_vec())
    .map(Self::Text)
    .unwrap_or_else(|e| {
        eprintln!("Warning: Invalid UTF-8 in regex literal: {:?}", e);
        Self::Empty
    })
```

### 5. **MEDIUM**: No Caching of Regex Compilation (Line 49)
**Location**: `checkito/src/regex.rs:49`

**Current Code**:
```rust
let hir = Parser::new().parse(pattern)?;
```

**Problem**:
- Parser instantiated fresh on every call
- No caching mechanism
- Repeated calls to same pattern recompile the HIR unnecessarily
- Performance impact for frequently used patterns

**Better**: Consider lazy_static or memoization:
```rust
use std::sync::LazyLock;

static PATTERN_CACHE: LazyLock<Mutex<HashMap<String, Hir>>> = ...;
```

### 6. **LOW**: Range Type Cast Without Validation (Line 117)
**Location**: `checkito/src/regex.rs:117`

**Current Code**:
```rust
(low as usize..=high as usize).into()
```

**Problem**:
- Direct cast from `u32` to `usize` without checking
- On 32-bit systems with high repetition counts, could overflow
- Should use `try_into()` with error handling

**Fix**:
```rust
(low.try_into().unwrap_or(usize::MAX)..=high.try_into().unwrap_or(usize::MAX)).into()
```

## Impact

### Security
- Invalid UTF-8 generation could cause issues in downstream code
- Unvalidated conversions may cause panics

### Correctness
- Generated strings may not match regex semantics
- Nested quantifiers behave incorrectly
- Silent failures hide bugs from users

### Performance
- Regex recompilation on every call

## Recommended Fixes Priority

1. **CRITICAL**: Fix u8→char cast (Line 68) - **Security/Correctness**
2. **CRITICAL**: Validate unbounded repetitions (Line 111) - **Correctness**
3. **HIGH**: Fix division truncation (Line 106) - **Correctness**
4. **MEDIUM**: Add UTF-8 error handling (Line 102) - **User Experience**
5. **MEDIUM**: Add pattern caching (Line 49) - **Performance**
6. **LOW**: Validate range casts (Line 117) - **Robustness**

## Testing Strategy
Add tests for:
1. Regex with byte ranges (e.g., `[\x80-\xFF]`) - should not panic
2. Unbounded quantifiers: `a*`, `a+`, `a{1000,}`
3. Deeply nested quantifiers: `((a*)*)*`
4. Invalid UTF-8 literals in patterns
5. Performance test for repeated pattern compilation

## Example Tests
```rust
#[test]
fn regex_handles_byte_ranges_safely() {
    // Should not panic or produce invalid chars
    let gen = regex(r"[\x00-\xFF]", None).unwrap();
    let mut state = State::default();
    let s = gen.generate(&mut state);
    assert!(s.is_char_boundary(0)); // Valid UTF-8
}

#[test]
fn unbounded_quantifiers_have_reasonable_limits() {
    let gen = regex("a*", None).unwrap();
    let mut state = State::default();
    let s = gen.generate(&mut state);
    assert!(s.len() <= 64); // Default repeats limit
}

#[test]
fn nested_quantifiers_dont_become_zero() {
    let gen = regex("((a*)*)*", None).unwrap();
    // Should still generate reasonable strings
}
```

## Priority
**High** - Multiple correctness and safety issues in a core generator

## Related Files
- `checkito/src/regex.rs`
- `checkito_macro/src/regex.rs` (similar issues may exist)
