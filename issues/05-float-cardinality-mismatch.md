# Issue: Float Cardinality Doesn't Match 2^n Bit Patterns

## Type
**Documentation / Clarification Needed**

## Severity
Low

## Description
The cardinality reported for `f32` and `f64` generators does not match the expected 2^32 and 2^64 bit patterns, respectively. The values are slightly less, suggesting that some bit patterns are being excluded.

## Observed Behavior
```
f32::generator() cardinality: Some(4278190080)
Expected (2^32):              4294967296
Difference:                   -16777216

f64::generator() cardinality: Some(18437736874454810624)
Expected (2^64):              18446744073709551616
Difference:                   -9007199254740992
```

## Analysis

### For f32:
- Difference: 16,777,216 = 2^24
- This is exactly 2^24, which is suspicious and likely not coincidental
- f32 has a 23-bit mantissa, so 2^24 could be related to NaN representations or denormal numbers

### For f64:
- Difference: 9,007,199,254,740,992 = 2^53
- This is exactly 2^53, which is the mantissa size + 1 for f64
- Again, likely related to NaN or special value handling

## Possible Explanations

### 1. NaN Canonicalization
IEEE 754 allows multiple bit patterns for NaN (any exponent of all 1s with a non-zero mantissa). The library might be:
- Treating all NaN patterns as a single value (cardinality of 1)
- Or excluding certain NaN patterns entirely

### 2. Denormal Numbers
Denormal (subnormal) numbers have special bit patterns and might be handled differently.

### 3. Signed Zero
`0.0` and `-0.0` are distinct bit patterns but compare as equal. The library might be treating them as one value.

## Impact
- **Documentation**: Users might be confused about why the cardinality doesn't match 2^n
- **Correctness**: If the library is excluding certain bit patterns, this should be documented
- **Testing**: When doing exhaustive testing with floats, users need to know what values are included/excluded

## Questions to Answer
1. **Is this intentional?** If so, what values are being excluded?
2. **Does generation produce all bit patterns?** Testing shows NaN and infinity are generated, but are all NaN patterns generated?
3. **Should the cardinality be documented?** Users need to understand what this number means.

## Recommendations

### If Intentional:
Document the behavior clearly:
```rust
/// Returns the cardinality of f32 values.
/// 
/// Note: This is less than 2^32 because:
/// - Multiple NaN bit patterns are treated as a single value
/// - (or other reason)
/// 
/// The exact count is determined by [specific calculation].
const CARDINALITY: Option<u128> = Some(4278190080);
```

### If Unintentional:
Fix the calculation to either:
- Include all 2^32 bit patterns (if that's the goal)
- Or explicitly document which patterns are excluded and why

## Investigation Needed
1. Check the source code for float cardinality calculation
2. Verify what values are actually generated
3. Test if all bit patterns can be generated
4. Document the design decision

## Test Cases
See `checkito/examples/float_cardinality_investigation.rs` for comprehensive tests.

## Additional Context
From testing, we know:
- NaN values ARE generated
- Infinity values (positive and negative) ARE generated
- The cardinality difference is EXACTLY a power of 2, suggesting systematic exclusion, not a calculation error
