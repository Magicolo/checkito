# Implement TODO: Weighted and Indexed Tuple Selection (State::any_tuple)

## Summary
The codebase has explicit TODOs indicating missing implementations for `any_tuple_indexed` and `any_tuple_weighted` in `state.rs`. Currently, only array-based weighted/indexed selection exists, limiting composability.

## Context
The `State` struct provides several `any_*` methods for selecting from multiple options:
- `any_array` - Select from array of generators
- `any_indexed` - Select from array with explicit index
- `any_weighted` - Select from array with weights
- **MISSING**: `any_tuple` variants for working with tuples

Tuples are fundamental in Rust for heterogeneous collections and are widely used in checkito's API (e.g., `(gen1, gen2, gen3).any()`).

## The TODO Comment

**Location**: `state.rs:260`
```rust
// TODO: Implement 'any_tuple_indexed' and 'any_tuple_weighted'...
```

**Location**: `any.rs:218`
```rust
// TODO: Use `State::any_tuple`
```

## Current Limitations

### 1. Homogeneous Arrays Only
Current `any_*` methods only work with arrays:
```rust
// state.rs lines 116-130
pub fn any_array<T, G: Generate<Item = T>, const N: usize>(
    &mut self,
    generators: &[G; N],
) -> (T, bool) { ... }

pub fn any_indexed<T, G: Generate<Item = T>, const N: usize>(
    &mut self,
    index: usize,
    generators: &[G; N],
) -> (T, bool) { ... }

pub fn any_weighted<T, G: Generate<Item = T>, const N: usize>(
    &mut self,
    generators: &[(Weight, G); N],
) -> (T, bool) { ... }
```

**Problem**: Arrays require all generators to have the **same type** and produce items of the **same type**.

### 2. No Tuple Support
Tuples allow heterogeneous generators:
```rust
// Desired but currently unsupported:
let generators = (
    number::<f64>(),      // Generates f64
    letter(),             // Generates char
    regex!("[0-9]+")      // Generates String
);

// Want: state.any_tuple(&generators)
// Currently: Must use arrays with type erasure or boxing
```

### 3. Workaround is Inefficient
Users currently must:
1. Box generators to erase types: `Box<dyn Generate<Item = Box<dyn Any>>>`
2. Use runtime type checking
3. Sacrifice type safety

**Example of current workaround** (inefficient):
```rust
// Must use any() combinator with Or types (any.rs:103-231)
let gen = (gen1, gen2, gen3).any();  // Returns Or3<G1, G2, G3>
// This works but produces complex nested Or types
```

## Proposed API

### any_tuple
```rust
impl State {
    pub fn any_tuple<T, G: Generate>(
        &mut self,
        generators: T,
    ) -> (G::Item, bool)
    where
        T: TupleGenerators<Output = G::Item>,
    { ... }
}
```

### any_tuple_indexed
```rust
impl State {
    pub fn any_tuple_indexed<T, G: Generate>(
        &mut self,
        index: usize,
        generators: T,
    ) -> (G::Item, bool)
    where
        T: TupleGenerators<Output = G::Item>,
    { ... }
}
```

### any_tuple_weighted
```rust
impl State {
    pub fn any_tuple_weighted<T, G: Generate>(
        &mut self,
        generators: T,
    ) -> (G::Item, bool)
    where
        T: TupleGeneratorsWeighted<Output = G::Item>,
    { ... }
}
```

### Supporting Trait
```rust
pub trait TupleGenerators {
    type Output;
    fn select(&self, state: &mut State, index: usize) -> (Self::Output, bool);
    fn len(&self) -> usize;
}

// Implement for (G1,), (G1, G2), (G1, G2, G3), ... up to reasonable arity
impl<G1: Generate> TupleGenerators for (G1,) { ... }
impl<G1: Generate, G2: Generate> TupleGenerators for (G1, G2) where G1::Item == G2::Item { ... }
// ... up to arity 12 or more
```

## Benefits

1. **Type Safety**: No boxing or type erasure needed
2. **Performance**: Direct function calls, no virtual dispatch
3. **Ergonomics**: Natural tuple syntax
4. **Consistency**: Matches Rust's tuple-based APIs
5. **Composition**: Works with existing combinators

## Use Cases

### Random Selection from Heterogeneous Generators
```rust
use checkito::*;

#[check(
    (0..10, 100..200, 1000..2000).any_tuple()
)]
fn test_any_range(value: i32) {
    // Value comes from one of three ranges
    assert!(value < 2000);
}
```

### Weighted Heterogeneous Selection
```rust
let generators = (
    (Weight(0.1), regex!("[a-z]+")),    // 10% - lowercase letters
    (Weight(0.3), regex!("[A-Z]+")),    // 30% - uppercase letters  
    (Weight(0.6), regex!("[0-9]+")),    // 60% - numbers
);

let (value, _) = state.any_tuple_weighted(&generators);
```

### Explicit Index Selection
```rust
let generators = (letter(), digit(), special());
let (value, _) = state.any_tuple_indexed(1, &generators); // Always selects digit
```

## Implementation Strategy

1. **Define tuple traits** (similar to existing macro-generated impls)
2. **Implement for tuples up to arity 12** (standard Rust library limit)
3. **Reuse existing weighted selection logic** from `any_weighted`
4. **Update `any.rs`** to use `State::any_tuple` (addresses TODO at line 218)

## Testing
Add tests for:
- Tuple selection with 2, 3, 5, 10 elements
- Weighted tuple selection with various weights
- Indexed tuple selection
- Edge cases: single-element tuples, all same type
- Verify shrinking works correctly
- Verify cardinality calculation

## Priority
**Medium** - This is a feature gap rather than a bug, but it's explicitly marked as TODO and would improve API ergonomics significantly.

## Related Issues
- TODOs at state.rs:260 and any.rs:218
- Related to overall API design and composability

## Estimated Effort
**Medium-Large** - Requires:
1. Trait design for tuple generators
2. Macro-generated implementations for various arities
3. Integration with existing `any()` combinator
4. Comprehensive testing
5. Documentation and examples
