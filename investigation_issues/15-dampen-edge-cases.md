# Dampen Edge Cases: Zero Limits Cause Abrupt Size Changes

## Summary
The `dampen` combinator in `dampen.rs` has edge case handling issues when `deepest=0` or `limit=0`, causing abrupt size changes from normal values to hardcoded `0.0` without gradual tapering.

## Context
The `dampen` combinator controls the size of nested structures by reducing the size parameter based on recursion depth and element count. This prevents infinite recursion and keeps generated data manageable.

## The Issue

**Location**: `checkito/src/dampen.rs:186-195`

```rust
let new = if with.state.depth >= deepest || with.state.limit >= limit {
    0.0  // Hardcoded!
} else {
    old.start() / utility::f64::max(with.state.depth as f64 * pressure, 1.0)
};
```

### Problem 1: Abrupt Jump to Zero
When threshold is reached, size immediately becomes `0.0`:
- **Before threshold**: Size might be 0.8, 0.6, 0.4, ...
- **At threshold**: Size becomes 0.0
- **No gradual tapering**: Discontinuous jump

**Example**:
```rust
let gen = number::<Vec<Vec<i32>>>()
    .dampen_with(3, 10, 1.0);  // deepest=3

// At depth 2: generates Vec with ~5 elements
// At depth 3: generates Vec with 0 elements (empty!)
// Very abrupt change
```

### Problem 2: Zero Deepest/Limit
**Edge Case**: What if user sets `deepest=0` or `limit=0`?

```rust
let gen = something().dampen_with(0, 0, 1.0);

// Line 186: depth >= 0 is always true!
// Line 186: limit >= 0 is always true!
// Size is ALWAYS 0.0
// Nothing gets generated
```

**Impact**: Effectively disables generation.

### Problem 3: Both Conditions Simultaneously
```rust
let gen = something().dampen_with(0, 100, 1.0);

// depth >= 0 is true immediately
// Size becomes 0.0
// limit threshold never reached
// limit parameter is ignored
```

**Issue**: No documentation on how these interact.

## Current Test Coverage

**Location**: `checkito/tests/prelude.rs:3-22`

**What's Tested**:
- `dampen_with(0, ..., ...)` forces minimal collections (line 10-16)
- `dampen_with(..., 0, ...)` forces minimal collections (line 18-24)

**What's NOT Tested**:
- Both zero simultaneously
- Gradual vs abrupt transitions
- Very deep nesting (depth > 10)
- Interaction with different pressure values
- Actual size values at different depths

## Recommended Improvements

### Option 1: Gradual Tapering (Better UX)
```rust
let new = if with.state.depth >= deepest || with.state.limit >= limit {
    // Instead of hard 0.0, taper gradually
    let overflow = f64::max(
        (with.state.depth.saturating_sub(deepest)) as f64,
        (with.state.limit.saturating_sub(limit)) as f64,
    );
    old.start() / (overflow + 1.0).exp() // Exponential decay
} else {
    old.start() / utility::f64::max(with.state.depth as f64 * pressure, 1.0)
};
```

**Benefits**:
- Smooth transition to small sizes
- No abrupt jumps
- Still enforces limits

### Option 2: Validate Parameters
```rust
pub fn dampen_with(
    generator: G,
    deepest: usize,
    limit: usize,
    pressure: f64,
) -> Dampen<G> {
    assert!(deepest > 0, "deepest must be > 0");
    assert!(limit > 0, "limit must be > 0");
    assert!(pressure > 0.0, "pressure must be > 0.0");
    // ...
}
```

**Benefits**:
- Catches user errors early
- Clear error messages
- Prevents edge cases

### Option 3: Document Behavior
```rust
/// # Edge Cases
///
/// - `deepest = 0`: All nested structures will be minimal (empty)
/// - `limit = 0`: All collections will be minimal (empty)
/// - Both zero: Nothing generated
/// - At threshold: Size becomes exactly 0.0 (no gradual tapering)
///
/// These behaviors are intentional to prevent infinite recursion,
/// but may cause abrupt size changes in generated data.
```

## Specific Edge Cases to Test

### Test 1: Both Zero
```rust
#[test]
fn dampen_both_zero_generates_minimal() {
    let gen = (0..100).collect::<Vec<_>>()
        .dampen_with(0, 0, 1.0);
    
    let value = gen.sample(1);
    assert_eq!(value.len(), 0); // Or minimal size
}
```

### Test 2: Size Continuity
```rust
#[test]
fn dampen_has_smooth_size_transition() {
    let gen = (0..100).collect::<Vec<_>>()
        .dampen_with(5, 100, 1.0);
    
    let mut sizes = Vec::new();
    for depth in 0..=6 {
        let mut state = State::default();
        state.depth = depth;
        let value = gen.generate(&mut state);
        sizes.push(value.len());
    }
    
    // Check that sizes don't jump abruptly
    for window in sizes.windows(2) {
        let ratio = window[1] as f64 / window[0].max(1) as f64;
        assert!(ratio > 0.1, "Size jumped too abruptly: {:?}", sizes);
    }
}
```

### Test 3: High Depth
```rust
#[test]
fn dampen_handles_very_deep_nesting() {
    let gen = number::<Vec<Vec<Vec<i32>>>>()
        .dampen_with(10, 100, 1.0);
    
    let mut state = State::default();
    state.depth = 50; // Very deep!
    
    let value = gen.generate(&mut state);
    // Should not panic or generate huge structures
}
```

### Test 4: Pressure Variations
```rust
#[test]
fn dampen_pressure_affects_size_correctly() {
    let low_pressure = something().dampen_with(5, 100, 0.5);
    let high_pressure = something().dampen_with(5, 100, 2.0);
    
    // High pressure should reduce size more aggressively
}
```

## Related Issues

### Interaction with Parallel Execution
**Question**: Is `State::depth` thread-safe when using `parallel()`?
- Each thread has own state?
- Or shared state with races?

**Needs Investigation**: `parallel.rs` + `dampen.rs` interaction.

### Interaction with Filter
**Question**: What if dampen makes size 0.0 but filter requires non-empty?
```rust
let gen = something()
    .dampen_with(0, 0, 1.0)  // Forces empty
    .filter(|v| !v.is_empty());  // Requires non-empty
    
// This will always fail filter!
// Infinite retry loop?
```

## Performance Considerations

### Current Calculation (Line 193)
```rust
old.start() / utility::f64::max(with.state.depth as f64 * pressure, 1.0)
```

**Issue**: Division on every generation call
**Minor**: Probably not a bottleneck, but could cache

## Priority
**Medium** - These are edge cases that can cause confusion, but the current behavior is at least deterministic and documented by tests.

## Related Code
- `checkito/src/dampen.rs`: Lines 186-195
- `checkito/tests/prelude.rs`: Lines 3-24 (existing tests)

## Acceptance Criteria
- [ ] Document edge case behavior (deepest=0, limit=0)
- [ ] Add tests for both zero simultaneously
- [ ] Add tests for size continuity
- [ ] Consider gradual tapering instead of hard cutoff
- [ ] Validate parameters or document why zeros are allowed
- [ ] Test interaction with filter and other combinators
- [ ] Test with parallel execution
