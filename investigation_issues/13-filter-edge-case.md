# Edge Case: Filter with Zero Retries Returns None Silently

## Summary
When `filter_with(0, predicate)` is used and all generated values fail the filter, the generator silently returns `None` without warning. This can lead to confusing test failures or unexpected behavior.

## Context
The filter combinators in checkito allow users to constrain generated values:
- `filter(predicate)` - Filter with default 256 retries
- `filter_with(retries, predicate)` - Filter with custom retry count

When all retries are exhausted without finding a matching value, the generator returns `None`.

## The Issue

**Locations**:
- `checkito/src/filter.rs:27-40`
- `checkito/src/filter_map.rs:27-40`

### Current Behavior
```rust
// filter.rs:27
let mut outer = None;
for i in 0..=self.retries {
    let item = self.generator.generate(state).item();
    if (self.filter)(&item) {
        outer = Some(item);
        break;
    }
}
// Returns None if no value matches after all retries
```

### Example of Silent Failure
```rust
use checkito::*;

// Filter that rejects EVERYTHING
let gen = (0..10).filter_with(0, |_| false);

let mut state = State::default();
let result = gen.generate(&mut state);

// result.item() returns None
// No warning, no panic, no indication that filter is broken
```

### Problem in Property Tests
```rust
#[check((0..100).filter_with(5, |x| x > 1000))]
fn test_large_numbers(x: i32) {
    // This will never run!
    // All generated values (0-99) fail the filter (> 1000)
    // Test silently passes because no values are generated
    assert!(x > 1000);
}
```

## Impact

### 1. Silent Test Passing
Tests that should fail may pass because no values are generated:
- User thinks property holds
- Actually, filter is misconfigured
- Bug in user's test logic goes undetected

### 2. Confusing Errors
When `None` propagates:
```rust
let value = gen.generate(&mut state).item().unwrap(); // Panics!
// Error: "called `Option::unwrap()` on a `None` value"
// User doesn't know why None was returned
```

### 3. No Diagnostic Information
Users don't know:
- How many retries were attempted
- How many values failed the filter
- What the last failed value was
- Whether filter is too strict or generator is incompatible

## Current Test Coverage

**Tests in `tests/filter.rs`**:
- Lines 51-59: Test shows `filter_map_with(0, ...)` can return `None`
- Lines 62-82: Test shows retries are attempted

**Gap**: No test for **warning users about misconfigured filters**

## Recommended Solutions

### Option 1: Warning Message (Conservative)
```rust
let mut outer = None;
for i in 0..=self.retries {
    let item = self.generator.generate(state).item();
    if (self.filter)(&item) {
        outer = Some(item);
        break;
    }
}

if outer.is_none() && self.retries > 0 {
    eprintln!("Warning: filter exhausted {} retries without finding a match", 
              self.retries + 1);
}

Shrinker { shrinker: outer, ... }
```

**Pros**: Non-breaking, informative
**Cons**: Users may not see stderr warnings

### Option 2: Panic on Exhaustion (Breaking Change)
```rust
let mut outer = None;
for i in 0..=self.retries {
    let item = self.generator.generate(state).item();
    if (self.filter)(&item) {
        outer = Some(item);
        break;
    }
}

let outer = outer.expect("filter exhausted all retries without finding a match");
```

**Pros**: Forces user to fix misconfigured filters
**Cons**: Breaking change, may be too strict

### Option 3: Configurable Behavior (Best)
```rust
pub enum FilterExhaustion {
    ReturnNone,      // Current behavior
    Warn,            // Print warning
    Panic,           // Panic with message
}

pub fn filter_with_config(
    retries: usize, 
    on_exhaustion: FilterExhaustion,
    filter: impl Fn(&T) -> bool
) -> Filter<Self, impl Fn(&T) -> bool> {
    // ...
}
```

**Pros**: Flexible, users choose behavior
**Cons**: More complex API

### Option 4: Track Statistics (Informative)
```rust
pub struct FilterStats {
    pub attempts: usize,
    pub matches: usize,
    pub exhausted: bool,
}

impl Filter {
    pub fn stats(&self) -> FilterStats { ... }
}
```

**Pros**: Users can inspect filter effectiveness
**Cons**: Requires storing state

## Specific Edge Cases to Handle

### Zero Retries
```rust
// retries = 0 means: try once, if it fails, return None
filter_with(0, predicate)
```
**Should**: Document that 0 retries still tries once (line 28: `0..=self.retries`)

### Impossible Filters
```rust
// Filter is impossible to satisfy
(0..10).filter(|x| x > 100)
```
**Should**: Warn after exhausting retries

### Very Strict Filters
```rust
// Filter passes 1% of the time
(0..100).filter_with(10, |x| x == 42)
```
**Should**: Consider statistical likelihood and warn if filter is too strict

## Documentation Improvements

### Current Documentation
`generate.rs:169-188` has good example for `filter()`, but doesn't mention:
- What happens when all retries fail
- How to choose appropriate retry count
- When to use `filter_with()`

### Should Add
```rust
/// # Edge Cases
///
/// If the filter predicate is too strict and rejects all generated values
/// within the retry limit, the generator returns `None`. This can cause
/// unexpected test behavior:
///
/// ```rust
/// # use checkito::*;
/// // This filter will never match values from 0..10
/// let gen = (0..10).filter_with(100, |x| x > 100);
/// 
/// let mut state = State::default();
/// assert_eq!(gen.generate(&mut state).item(), None);
/// ```
///
/// To avoid this:
/// - Ensure filter is compatible with generator
/// - Use sufficient retries for rare conditions
/// - Consider using `filter_map` to transform instead of filter
```

## Testing Strategy

Add tests:
```rust
#[test]
fn filter_warns_on_exhaustion() {
    // Capture stderr
    let gen = (0..10).filter_with(5, |_| false);
    // Should see warning in stderr
}

#[test]
fn filter_with_zero_retries_tries_once() {
    let gen = (0..10).filter_with(0, |x| x == 5);
    // Should try at least once
}

#[test]
#[should_panic(expected = "filter exhausted")]
fn impossible_filter_panics_in_strict_mode() {
    // If we add panic mode
    let gen = (0..10).filter_with_panic(10, |_| false);
    gen.generate(&mut State::default());
}
```

## Priority
**Medium** - This is an edge case that can cause confusion, but it's not a critical bug. The current behavior is documented by tests, but not well-documented for users.

## Related Code
- `checkito/src/filter.rs`: Lines 27-40
- `checkito/src/filter_map.rs`: Lines 27-40 (same issue)
- `checkito/src/generate.rs`: Lines 196-209 (documentation)
- `tests/filter.rs`: Lines 51-82 (existing tests)

## Acceptance Criteria
- [ ] Document behavior when retries exhausted
- [ ] Add warning or configuration option for exhaustion
- [ ] Add tests for edge cases
- [ ] Update user-facing documentation with examples
- [ ] Consider adding `FilterStats` for debugging
