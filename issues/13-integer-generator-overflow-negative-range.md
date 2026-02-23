# Issue: Integer Generator Overflow in Debug Mode for Negative Ranges at Full Size

## Summary

The random integer generator in `checkito/src/state.rs` can produce a signed
integer overflow in debug mode when generating from a negative range
(e.g. `i16::MIN..=0`) at full size (`size = 1.0`). The computation `end -
value` can overflow when `value` is the reinterpreted bit-cast of the unsigned
range bound. This panics in Rust debug builds, causing tests to fail
intermittently.

## Location

- `checkito/src/state.rs:587` — `end - value` in the `else if end <= 0` branch

```rust
Mode::Random(random) => {
    let range = shrink($positive::wrapping_sub(end as _, start as _), size, scale);
    let value = random.$positive(0..=range) as $integer;
    if start >= 0 {
        start + value
    } else if end <= 0 {
        end - value          // <-- potential overflow!
    } else { ... }
}
```

## Root Cause

For `i16` with range `[i16::MIN, 0]` (`$positive = u16`, `$integer = i16`):

1. `range = u16::wrapping_sub(0u16, 32768u16) = 32768u16`
   (the total number of values from `i16::MIN` to `0` is 32769, and
   `wrapping_sub` gives 32768 which is used as an inclusive upper bound)

2. `value = random.u16(0..=32768) as i16`
   When `random.u16` returns `32768`, the bit-cast is:
   `32768u16 as i16 = -32768i16` (= `i16::MIN`)

3. `end - value = 0i16 - (-32768i16) = +32768`
   This **overflows** `i16::MAX = 32767` and panics in Rust debug mode.

In **release mode**, the computation wraps modulo 2^16, giving `-32768i16`,
which happens to be the correct value (`i16::MIN = start`). The bug is
therefore **debug-mode only**, but still incorrect because the intended
semantics are wrapping arithmetic, not panic-on-overflow.

## Reproduction

```rust
// Run in debug mode (`cargo test`, NOT `cargo test --release`).
// The test is flaky because it requires random.u16 to return exactly 32768,
// which happens with probability 1/32769 ≈ 0.003%.
let result = (i16::MIN..=0i16).check(|value| value <= 0);
// Occasionally panics instead of returning None.
```

The failure was observed in the test `range::i16::is_negative` in
`checkito/tests/number.rs:148`.

## Impact

- **Flaky tests** in debug mode: `range::i16::is_negative` (and potentially
  similar tests for `i8`, `i32`, `i64`, `i128`, `isize`) can fail non-
  deterministically when the random seed produces the boundary value.
- The generator panic is **not caught** by `catch_unwind` in the checker
  (the panic is in `generator.generate()`, which is outside the checked
  region), so it propagates as an unhandled test failure.
- **Correct** in release mode only by accident (arithmetic wraps to the
  right value).

## Proposed Fix

Use `wrapping_sub` for the subtraction:

```rust
} else if end <= 0 {
    debug_assert!(start < 0);
    end.wrapping_sub(value)   // was: end - value
}
```

Analogously, verify the positive branch `start + value` for correctness. For
the positive branch, `value` is in `[0, end - start]` and `start + value` is
in `[start, end]`, which is always valid. However, for unsigned types (which
use `start >= 0` branch), `start + value` should also be checked for
correctness.

The `else { ... }` mixed-sign branch already uses `wrapping_add` / `wrapping_sub`
correctly (lines 595–597).

## Testing Strategy

1. Add a deterministic test:
   ```rust
   let mut state = State::random_with_seed(SEED_THAT_PRODUCES_32768);
   let value = state.i16(Range(i16::MIN, 0));
   assert_eq!(value, i16::MIN);
   ```
2. Run `negative::<i16>().check(|value| value <= 0)` 10 000 times with
   different seeds and confirm it always returns `None`.

## Related

- `checkito/src/state.rs:587` (overflow site)
- `checkito/tests/number.rs:148` (`range::i16::is_negative` — flaky test)
- The mixed-sign branch already uses `wrapping_add`/`wrapping_sub` correctly
  (state.rs:595-596), so the same pattern should be applied here.
