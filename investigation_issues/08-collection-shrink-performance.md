# Performance Issue: Triple Clone in Collection Shrinking

## Summary
The collection shrinking implementation in `collect.rs` performs three full vector clones on every shrinking iteration, causing O(n²) performance for large collections and making shrinking prohibitively expensive.

## Context
When a property test fails, checkito attempts to shrink the failing input to find the minimal failing case. For collections (Vec, String), this involves:
1. **Truncate**: Remove elements from the end
2. **Remove**: Try removing individual elements
3. **Shrink**: Shrink each remaining element

The current implementation clones the entire shrinker vector for each of these phases.

## The Performance Bug

**Location**: `checkito/src/collect.rs:115-165`

### Triple Clone Pattern

```rust
// Line 121 - Clone #1 in Truncate
let mut shrinkers = self.shrinkers.clone();
shrinkers.truncate(middle);

// Line 136 - Clone #2 in Remove
let mut shrinkers = self.shrinkers.clone();
shrinkers.remove(self.index);

// Line 154 - Clone #3 in Shrink
let mut shrinkers = self.shrinkers.clone();
```

**Problem**: Each shrink attempt creates a full copy of `Vec<S>` where `S` is the element shrinker type.

### Complexity Analysis

For a collection with `n` elements and `m` total shrink iterations:
- **Time**: O(n × m) for cloning alone
- **Space**: O(n) per shrink attempt
- **Total**: For removing elements one-by-one, this becomes **O(n²)** total clones

**Example**:
```rust
// Vec with 1000 elements, each needing 10 shrink steps
// Truncate phase: ~500 clones of 1000-element vectors = 500,000 clones
// Remove phase: ~1000 clones of 1000-element vectors = 1,000,000 clones
// Shrink phase: ~10,000 clones = 10,000,000 clones
// Total: ~11.5 million element clones!
```

## Additional Performance Issues

### No Capacity Pre-allocation (Line 99)
```rust
state.repeat(&self.generator, range).collect()
```

**Problem**: 
- `collect()` allocates incrementally
- Should pre-allocate with `.collect_vec_with_capacity(high)`
- Causes unnecessary reallocations during generation

### Linear Search in Shrink Phase
**Lines 149-161**: Shrinking iterates through all elements sequentially

**Problem**:
- No binary search for minimal failing element
- No weighted removal (all elements treated equally)
- Could use bisection to find failing elements faster

## Impact

### Real-World Performance
For a failing test with a Vec of 10,000 elements:
- Current: Seconds to minutes for shrinking
- With fix: Milliseconds to seconds

### User Experience
- Long shrinking times frustrate developers
- May hit timeouts in CI/CD pipelines
- Users may disable shrinking to speed up tests

### Memory Pressure
- Cloning large vectors creates GC/allocation pressure
- May trigger OOM on systems with limited memory

## Recommended Fixes

### 1. Use Persistent/Structural Sharing (Ideal)
```rust
// Instead of cloning, use indices/ranges
enum Machine {
    Truncate { end: usize },
    Remove { index: usize },
    Shrink { index: usize, inner: Box<S::Shrink> },
}

// Apply operations lazily without cloning
impl Machine {
    fn get_active_elements(&self, original: &[S]) -> impl Iterator<Item = &S> {
        match self {
            Truncate { end } => &original[..*end],
            Remove { index } => original.iter().enumerate()
                .filter(|(i, _)| i != index),
            Shrink { index, .. } => original.iter(),
        }
    }
}
```

### 2. Use Cow (Intermediate Solution)
```rust
use std::borrow::Cow;

// Only clone when necessary
let shrinkers: Cow<[S]> = if need_modification {
    Cow::Owned(self.shrinkers.clone())
} else {
    Cow::Borrowed(&self.shrinkers)
};
```

### 3. Pre-allocate Capacity (Quick Win)
```rust
// Line 99
state.repeat(&self.generator, range)
    .collect_with_capacity(high) // Add capacity hint
```

### 4. Binary Search for Shrinking (Algorithmic)
```rust
// Instead of linear shrinking, use binary search
// Find minimal subset that causes failure
fn binary_shrink(&mut self) -> Option<Vec<T>> {
    // Try removing half the elements
    // If still fails, recurse on that half
    // If passes, try other half
}
```

## Testing Strategy

### Performance Tests
```rust
#[test]
fn shrink_large_vec_performance() {
    use std::time::Instant;
    
    let start = Instant::now();
    let gen = (0..10000).collect::<Vec<_>>();
    gen.check(|v| v.len() < 5000); // Force shrinking
    let elapsed = start.elapsed();
    
    assert!(elapsed < Duration::from_secs(1), "Shrinking took too long");
}
```

### Correctness Tests
Ensure fixes don't break shrinking:
- Verify minimal failing cases still found
- Check that all shrink phases work correctly
- Test edge cases: empty vecs, single element, etc.

## Phased Implementation

### Phase 1: Quick Wins (Low Risk)
- Add capacity pre-allocation
- Profile to identify hottest clone sites

### Phase 2: Structural Changes (Medium Risk)
- Use Cow for conditional cloning
- Implement lazy operation application

### Phase 3: Algorithmic Improvements (High Risk)
- Binary search shrinking
- Weighted element removal
- Parallel shrinking exploration

## Priority
**High** - Performance issue that directly impacts user experience, especially for:
- Large data structures
- Complex nested collections
- Long-running test suites

## Related Code
- `checkito/src/collect.rs`: Lines 115-165 (shrinking logic)
- `checkito/src/collect.rs`: Line 99 (generation)
- `checkito/src/shrink.rs`: Shrink trait definition

## Acceptance Criteria
- [ ] Reduce clone operations by at least 50%
- [ ] Add performance regression tests
- [ ] Document shrinking complexity in rustdoc
- [ ] Measure and report performance improvements
