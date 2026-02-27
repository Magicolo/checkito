# Floating-Point Weighted Selection Can Panic at `unreachable!()`

## Summary

The `any_weighted` method on `State` and the `$w` (weighted-or) generated methods in
`state.rs` both contain `unreachable!()` branches that can be reached in practice due to
floating-point rounding errors, causing an unexpected panic.

## Affected Code

- `checkito/src/state.rs` – `State::any_weighted` (approximately line 260–276)
- `checkito/src/state.rs` – `or` macro, `$w` arm (approximately lines 825–838)

## Detailed Description

### `State::any_weighted`

```rust
Mode::Random(_) => {
    let total = generators
        .iter()
        .map(|Weight { weight, .. }| weight)
        .sum::<f64>()
        .min(f64::MAX);
    debug_assert!(total > 0.0 && total.is_finite());
    let mut random = self.with().size(1.0).f64(0.0..=total);
    debug_assert!(random.is_finite());
    for Weight { weight, generator } in generators {
        if random <= *weight {
            return Some(generator);
        } else {
            random -= weight;
        }
    }
    unreachable!();   // <-- can be reached!
}
```

The intent is: generate a uniform random value `r` in `[0, total]`, then walk through the
generators subtracting each weight until `r ≤ weight_i`, selecting generator `i`.  By
construction the last generator should always be selected if no earlier one was.

**The bug:** `total` is computed as the sum of all `f64` weights using floating-point
arithmetic.  The `f64` sum of `n` values can differ from the true mathematical sum.  For
example, `1/3 + 1/3 + 1/3` in `f64` evaluates to `1.0000000000000000`, and each `1/3` rounds
to `0.3333333333333333148...`.  After subtracting all three weights:

```
r = 1.0
r -= 1/3  => r ≈ 0.6666666666666667
r -= 1/3  => r ≈ 0.3333333333333334
r -= 1/3  => r ≈ 0.0000000000000001  (positive residual!)
```

On the final iteration `random` (≈ 1.1e-16) is NOT ≤ the last weight (≈ 0.333), so the loop
exits without selecting any generator and `unreachable!()` is triggered, panicking.

### Reproducer

```rust
use checkito::state::{State, Weight};
use checkito::generate::Generate;

let w = 1.0_f64 / 3.0;
let generators = [
    Weight::new(w, 0u8..=0),
    Weight::new(w, 1u8..=1),
    Weight::new(w, 2u8..=2),
];
// Run many times; the unreachable!() will eventually be hit.
for _ in 0..1_000_000 {
    state.any_weighted(&generators); // may panic
}
```

### Same Bug in the Tuple `or` Macro

The `$w` (weighted-or) variant in the `or!` macro has an identical pattern:

```rust
$(if _random <= $ps.weight {
    return orn::$n::Or::$ts($ps.generator);
} else {
    _random -= $ps.weight;
})*
unreachable!();
```

## Impact

- **Severity:** Medium – triggers a panic rather than incorrect results, but the panic is in
  library code and is non-obvious to users. The trigger probability increases with the number
  of generators and their weight magnitudes.
- Any use of `Weight` / weighted `any` tuples with three or more generators is at risk.

## Root Cause

Floating-point subtraction accumulates rounding error.  When `random` is sampled from
`[0.0, total]` and each weight is subtracted sequentially, a small positive residual can
remain after the last subtraction, preventing the final `≤` comparison from succeeding.

## Fix Plan

Replace the `unreachable!()` fallback in both locations with a fallback that selects the last
generator. Two equivalent approaches:

### Option A – Return the last generator as fallback

For `any_weighted`:
```rust
for Weight { weight, generator } in generators {
    if random <= *weight {
        return Some(generator);
    }
    random -= weight;
}
// Floating-point residual: fall back to the last generator.
generators.last().map(|w| &w.generator)
```

For the `or` macro, refactor to keep track of the last-seen generator and fall back to it:
```rust
let mut _last = None;
$(
    _last = Some(&$ps.generator);
    if _random <= $ps.weight {
        return orn::$n::Or::$ts($ps.generator);
    } else {
        _random -= $ps.weight;
    }
)*
// Floating-point residual: fall back to last generator.
// (This requires rethinking the macro since Or<..> variants have different types.)
```

Note: the tuple case is harder because each variant has a different type.  One approach is to
always return the last branch explicitly after the loop, keeping the types correct:

```rust
$(
    if _random <= $ps.weight {
        return orn::$n::Or::$ts($ps.generator);
    } else {
        _random -= $ps.weight;
    }
)*
// Guaranteed fallback – last generator takes any residual.
orn::$n::Or::$t_last($p_last.generator)
```

This requires the macro to emit the last generator outside of the loop. The cleanest
implementation would restructure the per-element `if` chain so that the last element is
always selected unconditionally.

### Option B – Clamp `random` to total before the loop

Before the loop, add `random = random.min(total)` (already guaranteed by generation, but
doesn't help due to accumulated subtraction error), and inside the loop for the last element
skip the comparison.

### Recommended Approach

Emit all but the last iteration as `if … return` blocks, then unconditionally return the last
generator. This approach is unambiguous and requires no floating-point adjustments.

## Test Cases to Add

```rust
#[test]
fn weighted_any_does_not_panic_with_equal_third_weights() {
    use checkito::state::{State, Weight};
    let w = 1.0_f64 / 3.0;
    let generators = [
        Weight::new(w, 0u8..=0),
        Weight::new(w, 1u8..=1),
        Weight::new(w, 2u8..=2),
    ];
    // Should not panic under any seed
    for seed in 0u64..1000 {
        let mut state = State::random(0, 1, Default::default(), seed);
        let _ = state.any_weighted(&generators);
    }
}

#[test]
fn weighted_tuple_does_not_panic_with_equal_third_weights() {
    use checkito::*;
    use checkito::state::Weight;
    let w = 1.0_f64 / 3.0;
    // Should not panic for any seed
    for seed in 0u64..1000 {
        let mut checker = (
            Weight::new(w, 0u8..=0),
            Weight::new(w, 1u8..=1),
            Weight::new(w, 2u8..=2),
        ).checker();
        checker.generate.seed = seed;
        checker.generate.count = 10;
        checker.check(|_| true);
    }
}
```
