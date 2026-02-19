# Issue: Lazy Generator Has Incorrect Static Cardinality

## Type
**Bug / Correctness Issue**

## Severity
Medium

## Description
The `lazy()` combinator returns `Some(1)` for its cardinality instead of delegating to the inner generator's cardinality. This is incorrect because a lazy generator should produce the same values as its inner generator, just with deferred initialization.

## Expected Behavior
```rust
let gen = lazy(|| 0u8..=10);
assert_eq!(gen.cardinality(), Some(11));
```

The lazy wrapper should transparently delegate cardinality to the inner generator.

## Actual Behavior
```rust
let gen = lazy(|| 0u8..=10);
println!("Cardinality: {:?}", gen.cardinality());
// Prints: Some(1)
// Expected: Some(11)
```

## Reproduction
```rust
use checkito::*;

fn main() {
    // Test with a simple range
    let gen = lazy(|| 0u8..=10);
    println!("lazy(|| 0u8..=10) cardinality: {:?}", gen.cardinality());
    // Prints: Some(1) -- WRONG!
    
    // Test with bool
    let gen = lazy(|| bool::generator());
    println!("lazy(|| bool) cardinality: {:?}", gen.cardinality());
    // Prints: Some(1) -- should be Some(2)
    
    // Test with a type that has None cardinality
    let gen = lazy(|| u128::generator());
    println!("lazy(|| u128) cardinality: {:?}", gen.cardinality());
    // Prints: Some(1) -- should be None
}
```

## Impact
- **Incorrect exhaustive testing**: If lazy is used to wrap a generator with known cardinality, exhaustive testing will fail or test the wrong number of cases.
- **Inconsistent behavior**: The lazy wrapper changes the observable cardinality of a generator, which violates the principle that it should be transparent.
- **Misleading documentation**: Users might avoid using lazy() if they need accurate cardinality, even though it should be safe.

## Root Cause
The issue appears to be in how `lazy` determines its static `CARDINALITY` constant. Looking at the repository memories:

```rust
const CARDINALITY: Option<u128> = G::CARDINALITY;

fn cardinality(&self) -> Option<u128> {
    self.0.get().map_or(G::CARDINALITY, G::cardinality)
}
```

The problem is that for the `lazy` type specifically, the **static** `CARDINALITY` cannot be determined at compile time because the inner generator is created by a closure. However, the **dynamic** `cardinality()` method should be able to call the closure and get the correct value.

## Why This Might Happen
`lazy` works with closures like `impl FnOnce() -> G`, and at the type level, we can't know what `G` is until runtime. This means:
- Static `CARDINALITY` might default to `Some(1)` (treating the closure itself as a single value)
- Dynamic `cardinality()` should force the closure and delegate to the inner generator

## Observed Behavior Analysis
The fact that we get `Some(1)` suggests that:
1. The static `CARDINALITY` is being used for the dynamic `cardinality()` call
2. OR the implementation treats an uninitialized lazy as having cardinality 1

## Expected Implementation
The dynamic `cardinality()` should:
```rust
fn cardinality(&self) -> Option<u128> {
    // Force the lazy value if not already initialized
    let inner = /* get or initialize inner generator */;
    inner.cardinality()
}
```

## Workaround
Don't use `lazy()` if you need accurate cardinality reporting. Instead, create the generator directly.

## Test Case
See `checkito/examples/wrapper_cardinality.rs`, function `test_lazy()`.

## Additional Investigation Needed
1. Check if other lazy-like wrappers (`boxed`, `rc`, `arc`) have the same issue
2. Determine if this is a fundamental limitation of lazy evaluation or an implementation bug
3. Consider whether the static `CARDINALITY` should be `None` for lazy generators to signal uncertainty
