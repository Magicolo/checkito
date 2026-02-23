# Issue: Primitive Integer Shrinker Does Not Always Reach Global Minimum

## Summary

The binary-search shrinker for integer types (`shrink!` macro in
`checkito/src/primitive.rs`) converges in O(log n) steps, which meets the
performance requirement, but it may not find the globally smallest failing
value because it follows a single binary-search path rather than exploring all
candidates.

## Location

- `checkito/src/primitive.rs:390–453` — `shrink!` macro
- Instantiated for all integer and float types via the `integer!` and
  `floating!` macros.

## Algorithm Description

For a positive failing value `item` in range `[start, end]`:

1. **Initial step** (`Direction::None`): clamp `start` to `0`; if `start ==
   item` we are done. Otherwise set `direction = High`, `end = item`, and emit
   a probe at `item = start` (i.e. try the lower boundary immediately).

2. **Binary search** (`Direction::High`): repeatedly bisect `[start, end]`
   toward `start` (smaller values), emitting the midpoint as the candidate.
   The current shrinker explores one half; the other half is the caller's
   responsibility to probe via future calls to `shrink()`.

For negative values the analogous `Direction::Low` path bisects toward `0`.

## Logarithmic Step Count ✓

For a range of size `N`, at most `1 + ceil(log2(N))` shrink steps are needed.
This satisfies the O(log n) requirement stated in the problem description.

**Example**: finding the minimum failing value for `value < 50` starting from
`item = 100` in `[0, 100]` takes approximately 8 steps.

## Minimality Limitation

The shrinker finds a *local* minimum along one binary-search path, not the
*global* minimum. For predicates where failing values are non-contiguous the
result may be larger than the true minimum.

**Example**: predicate `value % 2 == 0 && value > 5`; failing values are
`{6, 8, 10, 12, ...}`. Starting from `item = 10`:
- The shrinker may converge to `8` instead of `6` because the binary search
  can step over `6` depending on the exact midpoint sequence.

This is a known and accepted limitation of binary-search shrinking. Finding the
global minimum would require exhaustive search, which is O(n). The tradeoff is
deliberate.

## Float Shrinker Limitation

For floats, `Shrinker<f32/f64>::shrink` delegates to the same `shrink!` macro,
but only for finite values:

```rust
fn shrink(&mut self) -> Option<Self> {
    if self.item.is_finite() {
        shrink!(self, $type)
    } else {
        None
    }
}
```

Non-finite values (`NaN`, `±INFINITY`) are not shrunk at all. If a test fails
on `NaN` or `INFINITY`, the shrinker returns `None` immediately and the
checker reports the original non-finite value as the minimal failure. This
means:

- `NaN != NaN` predicates that fail for `NaN` are reported with `NaN` rather
  than a simpler value.
- `value == f32::INFINITY` failures cannot be simplified to, e.g., `f32::MAX`.

## Recommendations

1. **Document** the O(log n) guarantee and the non-global-minimum limitation
   in the `Shrink` trait documentation and in the module-level docs of
   `primitive.rs`.

2. **Float shrinker improvement**: when `item` is non-finite, attempt to
   shrink toward the nearest finite boundary. For `INFINITY`, try
   `f32::MAX`; for `NEG_INFINITY`, try `f32::MIN`; for `NaN`, try `0.0`.
   This would allow the shrinker to find simpler values in the common case
   where the test fails for `NaN` or infinity.

   ```rust
   fn shrink(&mut self) -> Option<Self> {
       if self.item.is_finite() {
           shrink!(self, $type)
       } else if self.item == $type::INFINITY {
           self.item = $type::MAX;
           self.end = $type::MAX;
           Some(Shrinker { item: $type::MAX, start: 0.0, end: $type::MAX, direction: Direction::None })
       } else if self.item == $type::NEG_INFINITY {
           ...
       } else {
           // NaN: try 0.0
           ...
       }
   }
   ```

3. **Test coverage**: add tests that confirm:
   - For `value < 50` on `0u8..=100`, the shrinker reaches `50` in ≤ 10 steps.
   - For `value.is_nan()` on `f32::generator()`, the shrinker emits at least
     one finite probe before returning the final failure.
   - For `value == f32::INFINITY`, the shrinker attempts `f32::MAX`.

## Related

- `checkito/src/primitive.rs:390–453` (`shrink!` macro)
- `checkito/tests/shrink.rs` (existing shrink tests)
