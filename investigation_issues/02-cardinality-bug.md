# Integer Cardinality Calculation Bug in primitive.rs

## Summary
The `cardinality()` method for integer range generators in `primitive.rs` has an off-by-one error that undercounts the total number of possible values in a range by 1.

## Context
Checkito generators implement a `cardinality()` method that returns the total number of distinct values the generator can produce. This is used for:
1. Determining whether exhaustive testing is feasible
2. Optimizing generation strategies
3. Accurate shrinking behavior

## The Bug

**Location**: `checkito/src/primitive.rs:656`

**Current Code**:
```rust
fn cardinality(&self) -> Option<u128> {
    Some(u128::wrapping_sub($type::MAX as _, $type::MIN as _))
}
```

**Problem**: This calculation is missing `.wrapping_add(1)` to account for the inclusive range `[MIN, MAX]`.

**Example**:
- For `u8`: `MAX = 255`, `MIN = 0`
- Current calculation: `255 - 0 = 255`
- Actual cardinality: `256` (values 0-255 inclusive)
- **Error**: Off by 1

**Another example**: 
- For `i8`: `MAX = 127`, `MIN = -128`
- Cast to u128: `127 as u128 - (-128 as u128)`
- Due to two's complement casting, this produces incorrect results
- Should be: `(127 - (-128)) + 1 = 256`

## Additional Issues

### Same Bug in char Range (Line 592)
```rust
fn cardinality(&self) -> Option<u128> {
    Some(u128::wrapping_sub(char::MAX as _, 0 as char as _))
}
```
- Missing `.wrapping_add(1)` 
- `char` range should include `char::MAX` in count

### Correct Implementation in Constant Ranges (Lines 279-283)
The code correctly handles this for constant ranges:
```rust
fn cardinality(&self) -> Option<u128> {
    u128::wrapping_sub(M as _, N as _).checked_add(1)
}
```
**This is the correct pattern** that should be used throughout.

### Signed Integer Casting Issue (Line 656)
For signed integers, direct casting can produce unexpected results:
```rust
// i8::MIN = -128, i8::MAX = 127
// This doesn't work correctly:
u128::wrapping_sub(127 as u128, -128 as u128)

// Should be:
((127 as i128) - (-128 as i128)) as u128 + 1
```

## Impact
1. **Exhaustive Testing**: May incorrectly determine that a generator is exhaustible when it's not, or vice versa
2. **Shrinking**: Cardinality affects shrinking strategies; incorrect values may produce suboptimal shrinking
3. **Correctness**: Property tests may report incorrect statistics about the input space

## Recommended Fix

### For Full Integer Types (Line 656)
```rust
fn cardinality(&self) -> Option<u128> {
    // For signed types, need proper conversion
    let min = $type::MIN as i128;
    let max = $type::MAX as i128;
    Some(((max - min) as u128).wrapping_add(1))
}
```

### For char Range (Line 592)
```rust
fn cardinality(&self) -> Option<u128> {
    u128::wrapping_sub(char::MAX as u32 as u128, 0).checked_add(1)
}
```

### For Integer Ranges (Line 338)
```rust
fn cardinality(&self) -> Option<u128> {
    let end = self.end() as u128;
    let start = self.start() as u128;
    end.checked_sub(start)?.checked_add(1)
}
```

## Testing
Add tests to verify:
1. `FullGenerate` for all integer types reports correct cardinality
2. `Range<u8, u8>` reports cardinality of 256
3. `Range<i8, i8>` reports cardinality of 256
4. `char` full range reports correct cardinality (valid Unicode code points)
5. Custom ranges like `10..=20` report cardinality of 11

## Priority
**High** - This is a correctness issue that affects core functionality (cardinality calculation is fundamental to the library's operation).

## Related Code
- `checkito/src/primitive.rs`: Lines 269-283 (correct pattern), 338, 592, 656
- `checkito/src/cardinality.rs`: Cardinality trait definition
