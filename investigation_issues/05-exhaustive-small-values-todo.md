# Implement TODO: Exhaustive Mode Should Generate Small Values First

## Summary
The exhaustive generation mode in `state.rs` currently generates values linearly across the entire range instead of prioritizing small values first. This breaks the fundamental property testing principle of testing edge cases and simple inputs before complex ones.

## Context
Property testing works best when:
1. Simple/small values are tested first (edge cases like 0, 1, -1)
2. Progressively larger values are tested
3. Edge cases are discovered early in the test run

The `Random` mode already implements this correctly by using size-based biasing (lines 573-598 in `primitive.rs`), but `Exhaustive` mode does not.

## The Problem

### Location 1: Integer Exhaustive Generation (state.rs:600-601)
**Current Code**:
```rust
Mode::Exhaustive(index) => consume(index, start as _, end as _) as $integer,
```

**Issue**: Generates values **uniformly** from `start` to `end`:
- For range `0..100`, generates: 0, 1, 2, 3, ..., 99, 100
- This treats value `50` with same priority as `0`
- Edge cases like `0`, `1`, `MAX` are not prioritized

**Compare with Random Mode** (primitive.rs:573-598):
```rust
Mode::Random(random) => {
    let progress = random.sample(with.state.progress());
    let size = with.state.size(with.sizes);
    // ... complex size-biased generation ...
}
```
Random mode biases toward smaller values early in testing.

### Location 2: Float Exhaustive Generation (state.rs:662-666)
**Current Code**:
```rust
Mode::Exhaustive(index) => utility::$number::from_bits(consume(
    index,
    utility::$number::to_bits(start) as _,
    utility::$number::to_bits(end) as _) as _),
```

**Issue**: Bit-level linear enumeration without size bias:
- Generates floats by incrementing bit representation
- Doesn't prioritize special values: `0.0`, `1.0`, `-1.0`, `MIN`, `MAX`, `EPSILON`
- Treats all floats equally instead of testing simple values first

## Impact on Test Quality

### Current Behavior (BAD):
```rust
// Exhaustive mode with range 0..1000, count=10
// Generates: 0, 100, 200, 300, 400, 500, 600, 700, 800, 900
// Misses important edge cases early!
```

### Desired Behavior (GOOD):
```rust
// Exhaustive mode should generate small values first
// Generates: 0, 1, -1, 2, -2, 10, -10, 100, -100, ...
// Discovers edge case bugs immediately
```

## Recommended Fix

### For Integers (state.rs:600-601):
```rust
Mode::Exhaustive(index) => {
    // Generate small values first using buckets/size-based ordering
    let size = calculate_size_from_index(index, count);
    let value = generate_within_size_bucket(start, end, size, index);
    value as $integer
}
```

**Strategy**: Use geometric buckets similar to exhaustive any selection (state.rs:325-348):
- First bucket: values near 0
- Second bucket: small values (< 10)
- Third bucket: medium values (< 100)
- Later buckets: full range

### For Floats (state.rs:662-666):
```rust
Mode::Exhaustive(index) => {
    // Prioritize special values and small magnitudes
    let size = calculate_size_from_index(index, count);
    generate_float_by_size(start, end, size, index)
}
```

**Strategy**: Order by magnitude, not bit representation:
- First: 0.0, ±1.0, ±EPSILON
- Then: small fractions like 0.5, 0.25
- Then: integers within range
- Finally: larger values

## TODO Comments

This issue addresses **2 explicit TODO comments** in the codebase:

**state.rs:600**:
```rust
// TODO: Generate 'small' values first. Maybe use the same adjustment as Random?
Mode::Exhaustive(index) => consume(index, start as _, end as _) as $integer,
```

**state.rs:662**:
```rust
// TODO: Generate 'small' values first. Maybe use the same adjustment as Random?
Mode::Exhaustive(index) => utility::$number::from_bits(consume(
```

## Testing Strategy
Create tests to verify:
1. Exhaustive mode generates `0` before large values
2. For range `-100..100`, generates: `0, 1, -1, 2, -2, ...`
3. Special float values (0.0, 1.0, EPSILON) appear early
4. Small-size property failures are discovered in first few test cases
5. Compare exhaustive vs random: both should find simple bugs quickly

## Example Test
```rust
#[test]
fn exhaustive_prioritizes_small_values() {
    let mut samples = Vec::new();
    let mut state = State::default();
    state.mode = Mode::Exhaustive(0);
    
    for i in 0..10 {
        state.mode = Mode::Exhaustive(i as u64);
        let value = state.integer::<i32>(0, 1000);
        samples.push(value);
    }
    
    // First values should be small
    assert!(samples[0].abs() < 10);
    assert!(samples[1].abs() < 10);
    assert!(samples[2].abs() < 10);
    
    // NOT linear: 0, 100, 200, 300, ...
    assert_ne!(samples, vec![0, 100, 200, 300, 400, 500, 600, 700, 800, 900]);
}
```

## Priority
**High** - This affects test quality for exhaustive mode, which is a core feature. Properties that fail on simple inputs should be discovered immediately.

## Related Issues
- Issue #14: "Add a way to check the full domain of the generator" - Exhaustive mode quality is critical for this feature

## Estimated Effort
**Medium** - Requires:
1. Design size-bucket algorithm for exhaustive mode
2. Implement for integers and floats separately
3. Add comprehensive tests
4. Ensure performance doesn't degrade
