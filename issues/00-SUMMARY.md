# Cardinality Feature Experimentation Summary

## Overview
This document summarizes the comprehensive experimentation conducted on the cardinality feature in the checkito library. The goal was to find edge cases, bugs, and areas where the static `Generate::CARDINALITY` or dynamic `Generate::cardinality()` methods do not produce expected values.

## Methodology
1. Created multiple experiment files in the `examples/` folder
2. Tested from the perspective of a library user (no access to internals)
3. Pushed the system to its limits with edge cases
4. Documented all findings as detailed GitHub issues in the `issues/` folder

## Experiments Conducted

### 1. `cardinality_experiments.rs`
- Comprehensive tests of primitive types (bool, integers, floats, chars)
- Range types with various boundaries
- Composite types (tuples, arrays, Option, Result)
- Combinators (map, any, collect)
- Overflow cases

### 2. `char_cardinality_deep_dive.rs`
- Detailed analysis of char cardinality
- Testing for surrogate code points
- Verification of actual generation vs. reported cardinality

### 3. `collection_cardinality.rs`
- Vec and collection cardinality with dynamic sizes
- String collections
- Nested collections
- **Filter combinator issue discovered**

### 4. `wrapper_cardinality.rs`
- Testing wrapper types: keep, dampen, flatten, lazy, boxed
- **Lazy cardinality issue discovered**

### 5. `lazy_investigation.rs`
- Deep dive into lazy generator cardinality
- Confirmed the bug with function pointer type inference

### 6. `math_edge_cases.rs`
- Mathematical overflow in products, sums, and powers
- Zero and one cardinality edge cases
- **Invalid range issue discovered**

### 7. `float_cardinality_investigation.rs`
- Analysis of f32 and f64 cardinality
- Special float values (NaN, infinity, denormals)
- Bit pattern analysis
- **Float cardinality mismatch documented**

### 8. `any_combinator_tests.rs`
- Basic any() combinator tests
- Overflow handling in sums
- Nested any() combinators
- Unify behavior

## Issues Found

### High Severity

#### Issue #02: Filter Cardinality Incorrect
**Type:** Bug / Correctness Issue  
**Severity:** High  
**Description:** The `filter()` combinator returns the cardinality of the underlying generator instead of `None`, even though the actual number of values that pass the filter cannot be determined statically.

**Impact:**
- Breaks fundamental correctness guarantee
- Would cause incorrect exhaustive testing
- Misleading metrics for any code relying on cardinality

**Example:**
```rust
let gen = Generate::filter(0u8..=10, |x| x % 2 == 0);
// Returns: Some(11)
// Should return: None
```

### Medium Severity

#### Issue #01: Char Cardinality Includes Surrogates
**Type:** Bug / Correctness Issue  
**Severity:** Medium  
**Description:** `char::generator()` reports a cardinality of `1,114,112`, which includes the 2,048 invalid surrogate code points `U+D800..=U+DFFF`.

**Impact:**
- Discrepancy between reported and actual cardinality
- Could affect exhaustive testing expectations
- Inconsistent with actual generation behavior

**Actual:** `1,114,112`  
**Expected:** `1,112,064`

#### Issue #03: Lazy Cardinality Incorrect
**Type:** Bug / Correctness Issue  
**Severity:** Medium  
**Description:** The `lazy()` combinator returns `Some(1)` for cardinality instead of delegating to the inner generator.

**Impact:**
- Breaks transparency of lazy wrapper
- Would fail exhaustive testing
- Users might avoid lazy() if they need accurate cardinality

**Example:**
```rust
let gen = lazy(|| 0u8..=10);
// Returns: Some(1)
// Should return: Some(11)
```

#### Issue #04: Invalid Range Cardinality
**Type:** Bug / Correctness Issue  
**Severity:** Medium  
**Description:** Ranges where end < start (e.g., `10u8..=0`) report a positive cardinality instead of zero.

**Impact:**
- Silent acceptance of invalid ranges
- Users can't rely on cardinality to validate ranges
- Inconsistent with standard library behavior

**Example:**
```rust
let gen = 10u8..=0;
// Returns: Some(11)
// Should return: Some(0)
```

### Low Severity (Documentation Needed)

#### Issue #05: Float Cardinality Mismatch
**Type:** Documentation / Clarification Needed  
**Severity:** Low  
**Description:** Float cardinality doesn't match the expected 2^32 and 2^64 bit patterns due to special handling of sign bits in the `to_bits` transformation.

**f32 Difference:** `-16,777,216` (exactly `-2^24`)  
**f64 Difference:** `-9,007,199,254,740,992` (exactly `-2^53`)

**Impact:**
- Potentially confusing for users
- Needs clear documentation
- May be intentional design decision

## Positive Findings

### What Works Well

1. **Overflow Handling:** The library correctly returns `None` for cardinality when operations would overflow u128.
   - Product overflow: `(u64, u64)` → `None`
   - Sum overflow: `(u128::MAX, u128::MAX).any()` handled correctly
   - Power overflow: `[u8; 17]` → `None`

2. **Collection Cardinality:** Correctly calculated for:
   - Vec with fixed and variable sizes
   - Strings
   - Nested collections
   - Uses geometric series formula for variable-length collections

3. **Any Combinator:** Correctly sums cardinalities
   - Handles overflow properly
   - Nested any() works correctly

4. **Wrapper Types (mostly):** Most wrappers correctly preserve cardinality:
   - `keep()` ✓
   - `dampen()` ✓
   - `boxed()` ✓
   - `map()` ✓

5. **Composite Types:** Tuples and arrays correctly multiply/exponentiate cardinalities

## Recommendations

### Immediate Actions Required

1. **Fix Filter Cardinality (High Priority)**
   - Change filter to return `None` for cardinality
   - Update tests and documentation

2. **Fix Lazy Cardinality (Medium Priority)**
   - Ensure lazy wrapper correctly delegates cardinality
   - May need to force evaluation or change how static CARDINALITY is computed

3. **Fix Invalid Range Handling (Medium Priority)**
   - Return `Some(0)` for invalid ranges
   - Or consider panicking/error for invalid ranges
   - Align with standard library behavior

4. **Fix Char Cardinality (Medium Priority)**
   - Exclude surrogate code points from calculation
   - Update to return `1,112,064`

### Documentation Improvements

1. **Document Float Cardinality**
   - Explain why it's less than 2^n
   - Document the sign bit transformation
   - Clarify what values are included/excluded

2. **Add Cardinality Examples**
   - Show edge cases in documentation
   - Explain None vs Some(n) semantics
   - Document behavior of each combinator

3. **Testing Guidance**
   - When to rely on cardinality for exhaustive testing
   - When cardinality is unavailable (None)
   - Best practices for using cardinality

## Test Coverage

All experiments were originally implemented in `checkito/examples/` with detailed comments and assertions. The following commands were used to run them during development:

```bash
cargo run --example cardinality_experiments
cargo run --example char_cardinality_deep_dive
cargo run --example collection_cardinality
cargo run --example wrapper_cardinality
cargo run --example math_edge_cases
cargo run --example float_cardinality_investigation
cargo run --example any_combinator_tests
```

## Conclusion

The cardinality feature is generally well-implemented with good overflow handling and correct behavior for most types and combinators. However, there are **4 bugs** that should be fixed:

1. Filter cardinality (High)
2. Lazy cardinality (Medium)
3. Invalid range cardinality (Medium)
4. Char cardinality with surrogates (Medium)

And **1 area needing documentation**:
1. Float cardinality calculation

Once these issues are addressed, the cardinality feature will be more robust and reliable for users.
