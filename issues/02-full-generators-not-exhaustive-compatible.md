# Issue: Full<T> and Standard Generators Are Not Exhaustive-Mode Compatible

## Summary

The `Full<T>` generators for integers, floats, and the standard `Option` and
`Result` generators use a random `u8` branch selector to dispatch between
sub-generators. In exhaustive mode this wastes part of the exhaustive index on
the branch decision, producing non-deterministic and incomplete coverage of the
value space.

## Location

Every location carries the comment `// TODO: Will this work properly in
exhaustive mode?`

| File | Line | Type |
|------|------|------|
| `checkito/src/primitive.rs` | 594 | `Full<char>::generate` |
| `checkito/src/primitive.rs` | 683 | `Full<$integer>::generate` |
| `checkito/src/primitive.rs` | 775 | `Full<$float>::generate` |
| `checkito/src/standard.rs` | 41 | `option::Generator<G>::generate` |
| `checkito/src/standard.rs` | 113 | `result::Generator<T,E>::generate` |

## Root Cause

### Example — `Full<i32>::generate` (primitive.rs:683)

```rust
fn generate(&self, state: &mut State) -> Self::Shrink {
    // TODO: Will this work properly in exhaustive mode?
    let value = state.with().size(1.0).u8(..);
    match value {
        0..=249 => Range(i32::MIN, i32::MAX).generate(state),
        250..   => Shrinker {
            item: Special::<i32>::VALUE.generate(state),
            ...
        },
    }
}
```

`state.with().size(1.0).u8(..)` advances the exhaustive index by consuming a
value from the `[0, 255]` range. After this call the remaining index is
`original_index / 256`. The 250-255 band (special values: `0`, `MIN`, `MAX`)
is only reached when the index modulo 256 falls in `250..=255` — a 2.3 %
slice. All other indices go to the full range branch, and the special-values
branch may never be reached in short exhaustive runs.

The same pattern appears in `Full<f32>`, `Full<f64>`, `Full<char>`,
`option::Generator`, and `result::Generator`.

### What exhaustive mode expects

In exhaustive mode the index should be deterministically mapped to a unique
value. Consuming the index piecemeal with a branch selector breaks this
contract: the index consumed by the branch selector "overlaps" with the index
consumed by the sub-generator, producing duplicate or missing values.

## Impact

- `Full<i32>` (and all other `Full` types) in exhaustive mode will not cover
  special values (`0`, `MIN`, `MAX`) reliably.
- `Option<G>` in exhaustive mode will produce `None` only ~50 % of the time
  rather than exactly once per two iterations.
- `Result<T, E>` in exhaustive mode will not interleave `Ok` and `Err` evenly.
- Any test that relies on exhaustive mode to cover all values of a `Full`
  generator will silently miss cases.

## Proposed Fix

Use `State::any_exhaustive` (already implemented in `state.rs:116`) to select
the branch deterministically:

```rust
fn generate(&self, state: &mut State) -> Self::Shrink {
    match &mut state.mode {
        Mode::Exhaustive(index) => {
            // Deterministically choose between normal range and special values.
            // Special values have cardinality 3 (0, MIN, MAX for integers).
            let generators: [Box<dyn Generate<Item=i32, Shrink=Shrinker<i32>>>; 2] = [
                Box::new(Range(i32::MIN, i32::MAX)),
                Box::new(Special::<i32>::VALUE),
            ];
            State::any_exhaustive(index, generators)
                .unwrap()
                .generate(state)
        }
        _ => { /* existing random branch */ }
    }
}
```

Alternatively, give the special branch a fixed cardinality (3 for integers, 8
for floats) and interleave it with the range branch using `any_exhaustive`.

## Testing Strategy

1. In exhaustive mode with `count = 259`, `Full<i32>` should produce all three
   special values (`0`, `i32::MIN`, `i32::MAX`) exactly once.
2. `Option<bool>` in exhaustive mode with `count = 3` should produce
   `[None, Some(false), Some(true)]` (or a permutation).
3. `Result<bool, bool>` in exhaustive mode with `count = 4` should produce all
   four combinations exactly once.

## Related

- `checkito/src/primitive.rs:594, 683, 775` (TODO comments)
- `checkito/src/standard.rs:41, 113` (TODO comments)
- Issue 01 (exhaustive mode small values first) — closely related
