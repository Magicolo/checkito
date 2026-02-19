# Issue: Char Generator Cardinality Includes Invalid Surrogate Code Points

## Type
**Bug / Correctness Issue**

## Severity
Medium

## Description
The `char::generator()` reports a cardinality of `1,114,112`, which includes the surrogate code point range `U+D800` to `U+DFFF` (2,048 invalid code points). However, the actual generator correctly avoids generating these invalid code points by mapping them to `REPLACEMENT_CHARACTER`.

This creates a discrepancy between the reported cardinality and the actual number of unique values that can be generated.

## Expected Behavior
The cardinality should be `1,112,064` (excluding the 2,048 surrogate code points):
- Total Unicode code points: `1,114,112` (from `U+0000` to `U+10FFFF`)
- Surrogate code points: `2,048` (from `U+D800` to `U+DFFF`)
- Valid char values: `1,114,112 - 2,048 = 1,112,064`

## Actual Behavior
```
char::generator() cardinality: Some(1114112)
```

## Reproduction
```rust
use checkito::*;

fn main() {
    let gen = char::generator();
    println!("char cardinality: {:?}", gen.cardinality());
    // Prints: Some(1114112)
    // Expected: Some(1112064)
    
    // Verify actual generation doesn't produce surrogates
    let samples: Vec<char> = gen.samples(10000).collect();
    for c in &samples {
        let code_point = *c as u32;
        assert!(code_point < 0xD800 || code_point > 0xDFFF, 
                "Generated invalid surrogate: {:?}", c);
    }
}
```

## Impact
- **Exhaustive testing**: If a user relies on the cardinality for exhaustive testing, they might think they're testing 1,114,112 cases when they're actually only testing 1,112,064.
- **Documentation**: The cardinality value in documentation or error messages would be misleading.
- **Consistency**: Creates inconsistency between what the cardinality promises and what the generator delivers.

## Root Cause
The cardinality calculation for `char` uses a naive formula:
```rust
(char::MAX as u128) - (0 as u128) + 1 = 1,114,112
```

This doesn't account for the surrogate gap in valid Unicode code points.

## Suggested Fix
The cardinality calculation for `char` should exclude surrogates:
```rust
const CHAR_CARDINALITY: u128 = 
    ((0xD800u32) - 0) +           // U+0000 to U+D7FF
    ((0x10FFFFu32 + 1) - 0xE000); // U+E000 to U+10FFFF
// = 55,296 + 1,056,768 = 1,112,064
```

## Additional Notes
Similar issues might exist for character ranges that span the surrogate range:
```rust
let gen = '\u{D7FF}'..='\u{E000}';
println!("Cardinality: {:?}", gen.cardinality());
// Currently: Some(2050) (naive calculation)
// Should be: Some(2) (only U+D7FF and U+E000 are valid)
```

## Test Case
See `checkito/examples/char_cardinality_deep_dive.rs` for comprehensive test.
