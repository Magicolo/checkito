# Issue: Adaptive Test-Count Based on Runtime Duration

## Summary

The `Generates` configuration struct uses a fixed `count` (default 1024) for
the number of test cases. There is a TODO suggesting that the count should
instead be determined dynamically based on observed generation and check runtime.

## Location

- `checkito/src/lib.rs:131`

```rust
/*
    TODO:
    - Instead of running a fixed number of checks, determine the number of checks
      based on the runtime of the generation and check.
*/
```

## Motivation

Property testing quality scales with the number of test cases. With a fixed
count, fast properties get 1024 iterations regardless of whether 10 000 would
complete in the same wall-clock time, while slow properties complete in
`count * check_time` regardless of a time budget.

A time-budget approach would:
- Run fast properties more thoroughly (more iterations in the same time).
- Cap expensive properties automatically.
- Make CI times more predictable.

## Proposed Design

Add an optional `duration` field to `Generates`:

```rust
pub struct Generates {
    pub count: usize,
    pub duration: Option<std::time::Duration>,  // NEW
    // ... existing fields ...
}
```

When `duration` is `Some(d)`, the checker runs until either `count` iterations
are complete or `d` has elapsed (whichever comes first). A calibration phase
(e.g. 10 warm-up iterations) can estimate per-iteration cost and set an
adaptive count.

## Considerations

- This requires access to `std::time::Instant`, which adds a `std` dependency
  to the hot path. The feature should be opt-in or gated behind a feature flag.
- Reproducibility: if the count varies between runs, failures may not
  reproduce. The final count should be reported alongside the seed so users can
  reproduce with `checker.generate.count = <reported_count>`.
- Exhaustive mode is not affected (the count is derived from cardinality).

## Investigation Required

1. Decide whether this is a new feature or a change to existing defaults.
2. Prototype the time-sampling approach and measure overhead on a simple
   benchmark.
3. Determine whether `no_std` compatibility is impacted.

## Related

- `checkito/src/lib.rs:131` (TODO comment)
- `checkito/src/check.rs` — `Generates` struct
