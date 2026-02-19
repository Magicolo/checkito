# Issue: Invalid Range (End < Start) Has Non-Zero Cardinality

## Type
**Bug / Correctness Issue**

## Severity
Medium

## Description
When creating a range where the end is less than the start (e.g., `10u8..=0`), the cardinality calculation returns a positive value instead of zero. This is incorrect because such a range is invalid and contains no values.

## Expected Behavior
```rust
let gen = 10u8..=0;
assert_eq!(gen.cardinality(), Some(0));
// Or perhaps the library should panic/error on invalid ranges
```

## Actual Behavior
```rust
let gen = 10u8..=0;
println!("Cardinality: {:?}", gen.cardinality());
// Prints: Some(11)
// This is the same as 0u8..=10, treating it as |10 - 0| + 1
```

## Reproduction
```rust
use checkito::*;

fn main() {
    // Invalid range: end < start
    let gen = 10u8..=0;
    println!("10u8..=0 cardinality: {:?}", gen.cardinality());
    // Prints: Some(11)
    
    // Another example
    let gen = 100i32..= -100;
    println!("100i32..= -100 cardinality: {:?}", gen.cardinality());
    // What does this print?
    
    // Edge case: far apart values
    let gen = u8::MAX..=u8::MIN;
    println!("u8::MAX..=u8::MIN cardinality: {:?}", gen.cardinality());
    // Prints: Some(256) - same as valid range!
}
```

## Impact
- **Incorrect cardinality**: Users might rely on cardinality to determine if a range is valid, but this would give the wrong answer.
- **Silent errors**: The library accepts invalid ranges without error, which could lead to confusion.
- **Unexpected behavior**: Users might accidentally create invalid ranges and not realize it because the cardinality suggests it's valid.

## Possible Behaviors to Consider

### Option 1: Return Some(0)
Invalid ranges have zero cardinality:
```rust
let gen = 10u8..=0;
assert_eq!(gen.cardinality(), Some(0));
```

### Option 2: Panic on Creation
Reject invalid ranges at creation time:
```rust
let gen = 10u8..=0;  // Panics: "invalid range: end < start"
```

### Option 3: Swap Internally
Treat ranges as unordered and normalize them:
```rust
let gen = 10u8..=0;
// Internally becomes: 0u8..=10
assert_eq!(gen.cardinality(), Some(11));
```

### Option 4: Current Behavior (Document It)
If the current behavior is intentional (taking absolute difference), it should be clearly documented.

## Root Cause
Looking at the cardinality calculation for ranges, it likely uses:
```rust
wrapping_sub(end, start).wrapping_add(1)
// or
if end < start { start - end } else { end - start } + 1
```

This treats the range as absolute distance rather than a directed range.

## Standard Library Behavior
Rust's standard library `RangeInclusive` treats invalid ranges as empty:
```rust
let range = 10u8..=0;
assert_eq!(range.count(), 0);  // Iterator produces nothing
```

The checkito library should probably match this behavior for consistency.

## Suggested Fix
Modify the cardinality calculation to:
```rust
if end < start {
    Some(0)  // or None to indicate invalid
} else {
    // normal calculation
    wrapping_sub(end, start).checked_add(1)
}
```

## Related Issues
This might affect:
- Generation: Does the generator produce any values for invalid ranges?
- Shrinking: How does shrinking work for invalid ranges?
- Documentation: Should the API prevent invalid ranges at the type level?

## Test Case
See `checkito/examples/math_edge_cases.rs`, function `test_zero_cardinality()`.
