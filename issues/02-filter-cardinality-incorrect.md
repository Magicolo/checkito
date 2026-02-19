# Issue: Filter Combinator Returns Underlying Cardinality Instead of None

## Type
**Bug / Correctness Issue**

## Severity
High

## Description
The `filter()` combinator returns the cardinality of the underlying generator instead of `None`, even though the actual number of values that pass the filter cannot be determined statically.

This creates a fundamental correctness issue: the reported cardinality does not match the actual number of unique values the generator can produce.

## Expected Behavior
Since a filter predicate can reject any subset of values from the underlying generator, and this cannot be determined at compile time, the cardinality should be `None`:

```rust
let gen = (0u8..=10).filter(|x| x % 2 == 0);
assert_eq!(gen.cardinality(), None);
```

## Actual Behavior
```rust
let gen = (0u8..=10).filter(|x| x % 2 == 0);
println!("Cardinality: {:?}", gen.cardinality());
// Prints: Some(11)
// Expected: None
```

## Reproduction
```rust
use checkito::*;

fn main() {
    // Filter that keeps even numbers
    let gen = Generate::filter(0u8..=10, |x| x % 2 == 0);
    println!("Filter(even) cardinality: {:?}", gen.cardinality());
    // Prints: Some(11) -- WRONG!
    // Should be: None
    
    // The actual cardinality is 6 (0, 2, 4, 6, 8, 10)
    // but we report 11!
    
    // Even worse - a filter that rejects everything
    let gen = Generate::filter(0u8..=10, |_| false);
    println!("Filter(reject all) cardinality: {:?}", gen.cardinality());
    // Prints: Some(11) -- VERY WRONG!
    // Actual cardinality is 0, should report None
}
```

## Impact
- **Incorrect exhaustive testing**: If a user relies on cardinality for exhaustive testing, they will test the wrong number of cases.
- **Misleading metrics**: Any code that uses cardinality for estimation or reporting will be incorrect.
- **Breaking invariants**: The cardinality no longer represents the number of unique values the generator produces.

## Comparison with Other Combinators
Most other combinators correctly handle cardinality:
- `map`: Preserves cardinality ✓ (same number of values, just transformed)
- `keep`: Preserves cardinality ✓ (wrapper doesn't change values)
- `dampen`: Preserves cardinality ✓ (only affects shrinking)
- `collect`: Correctly calculates based on count range ✓
- `filter`: **Incorrectly preserves cardinality** ✗

## Root Cause
Looking at the implementation, `filter` likely passes through the underlying generator's cardinality without accounting for the predicate:

```rust
// Likely current implementation:
const CARDINALITY: Option<u128> = G::CARDINALITY;

fn cardinality(&self) -> Option<u128> {
    self.generator.cardinality()  // Wrong!
}
```

## Suggested Fix
Filter should always return `None` for cardinality:

```rust
const CARDINALITY: Option<u128> = None;

fn cardinality(&self) -> Option<u128> {
    None
}
```

## Alternative Considerations
One might argue that we could provide an upper bound (the underlying cardinality), but this would be misleading because:
1. The actual cardinality could be anywhere from 0 to the underlying cardinality
2. Users might misinterpret this as the exact cardinality
3. It's better to be explicit that we don't know than to provide a potentially very wrong estimate

## Related Combinators
Other combinators that might have similar issues:
- `filter_map`: Should also return None
- Any combinator that can reject values dynamically

## Test Case
See `checkito/examples/collection_cardinality.rs`, function `test_filter_cardinality()`.
