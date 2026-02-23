# Issue: filter and filter_map Do Not Work Correctly in Exhaustive Mode

## Summary

The `Filter` and `FilterMap` generators use a retry loop that increments the
`size` ratio on each attempt. In exhaustive mode this advances the exhaustive
index multiple times per generated value, causing items to be skipped and
breaking the deterministic coverage guarantee.

## Location

- `checkito/src/filter.rs:29` — `Filter::generate`
- `checkito/src/filter_map.rs:29` — `FilterMap::generate`

Both carry:
```rust
// TODO: Will this work properly in exhaustive mode?
let sizes = Sizes::from_ratio(i, self.retries, state.sizes());
let inner = self.generator.generate(state.with().sizes(sizes).as_mut());
```

## Root Cause

```rust
fn generate(&self, state: &mut State) -> Self::Shrink {
    let mut outer = None;
    for i in 0..=self.retries {         // up to RETRIES+1 = 257 iterations
        let sizes = Sizes::from_ratio(i, self.retries, state.sizes());
        let inner = self.generator.generate(state.with().sizes(sizes).as_mut());
        ...
    }
    ...
}
```

Each call to `generator.generate(state)` in exhaustive mode advances the
internal exhaustive `index` by dividing it by the sub-generator's cardinality.
When the filter fails on the first attempt and succeeds on the second, two
index steps have been consumed. This means that the *n*-th call to
`Filter::generate` does not correspond to the *n*-th exhaustive index — the
mapping is non-deterministic and dependent on how many retries were needed.

For generators with low acceptance rates the index advances very quickly,
exhausting the entire value space long before `count` iterations are completed.

## Impact

- `Filter` and `FilterMap` with low acceptance rates in exhaustive mode will
  silently skip large portions of the value space.
- Coverage guarantees that hold for random mode do not hold for exhaustive
  mode when these combinators are used.
- The `CARDINALITY` reported by `Filter` / `FilterMap` (`G::CARDINALITY`)
  overestimates the number of unique values that will be produced by the
  filtered generator, which can cause the checker to switch to exhaustive mode
  unnecessarily.

## Proposed Fix

In exhaustive mode, use a single generate call without retries:

```rust
fn generate(&self, state: &mut State) -> Self::Shrink {
    match state.mode {
        Mode::Exhaustive(_) => {
            // In exhaustive mode, generate exactly one value per index step.
            // Do not retry; the caller will iterate to the next index naturally.
            let inner = self.generator.generate(state);
            let item = inner.item();
            let accepted = (self.filter)(&item);
            Shrinker {
                shrinker: if accepted { Some(inner) } else { None },
                filter: self.filter.clone(),
            }
        }
        _ => { /* existing retry loop */ }
    }
}
```

For `CARDINALITY` reported by `Filter`, the correct value is unknown without
running the filter, so it should either remain as `G::CARDINALITY` (an upper
bound) or be changed to `None` to signal "unknown".

## Investigation Required

1. Decide whether `Filter::CARDINALITY` should be `None` (unknown) or
   `G::CARDINALITY` (upper bound). The current choice (`G::CARDINALITY`) can
   trigger exhaustive mode when the effective cardinality is much lower,
   leading to repeated duplicate-or-None outputs.
2. Determine whether the single-generate approach in exhaustive mode is
   semantically correct or whether some retry mechanism is still needed.

## Testing Strategy

1. `(0u8..=9).filter(|&x| x % 2 == 0)` in exhaustive mode should produce
   exactly `[Some(0), Some(2), Some(4), Some(6), Some(8), None, None, ...]`
   (or similar deterministic sequence) across 10 iterations.
2. The same generator in random mode with retries should still filter correctly.

## Related

- `checkito/src/filter.rs:29` (TODO comment)
- `checkito/src/filter_map.rs:29` (TODO comment)
- Issue 02 (Full<T> not exhaustive compatible)
