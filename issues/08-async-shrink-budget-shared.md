# Issue: Async Checker Shrink Budget Is Shared Across All Candidates

## Summary

The asynchronous property checker (`check::asynchronous::Stream`) uses the
ring-buffer `tail` counter as the shrink-step index. Because `tail` is never
reset when a better (simpler) failing value is found, the shrink budget is
shared across all shrink candidates rather than being per-candidate. This can
cause the shrinker to terminate prematurely when several better failures are
found in sequence, each consuming part of the global budget.

## Location

- `checkito/src/check.rs` — `Machine::Shrink` arm of `Stream::poll_next`

Key lines:
```rust
fill(
    this.head, this.tail, this.entries.as_mut(), &mut this.check,
    |index| {
        if index < *this.shrinks {    // `index` is *this.tail, which never resets
            let shrinker = old_shrinker.shrink()?;
            ...
        } else {
            None
        }
    },
);
```

And on finding a better failure:
```rust
*this.head = *this.tail;   // reset buffer, but NOT tail itself
*this.machine = Machine::Shrink { shrinker: new_shrinker, ... };
```

## Comparison with Synchronous Checker

In the synchronous checker (`synchronous::Iterator`), the shrink budget is
also shared across candidates — `shrinks: ops::Range<usize>` persists across
candidate switches. When a new failure is found at step `k`, shrinking
continues from step `k` rather than from `0`. This is intentional: the total
budget is `SHRINKS` steps regardless of how many sub-candidates are explored.

The async checker replicates this behaviour via the `tail` counter: `tail`
increments for every shrink generated (just as `shrinks.start` advances for
every shrink tested in the sync path). The difference is that with
`concurrency > 1`, `tail` can advance ahead of the resolved index when the
buffer is partially filled, potentially over-counting shrink steps.

## Specific Concern: concurrency > 1 and buffer reset

When a better failure is found:

1. `*this.head = *this.tail` — the buffer is logically cleared.
2. The machine is set to `Machine::Shrink` with the new (simpler) shrinker.
3. On the next `poll_next` call, `fill` is invoked again starting at
   `index = *this.tail`.

If `concurrency = 4` and 3 in-flight shrinks were discarded at step `tail = 7`,
those 3 steps are "wasted": the new candidate's budget starts at index 7, not
0. The new candidate gets `SHRINKS - 7` remaining steps rather than `SHRINKS`.

For the default `SHRINKS = 1 << 20` (1 048 576) this is unlikely to matter in
practice. However, with a small configured `shrink.count` (e.g. 16) and high
concurrency, the effective per-candidate budget can be significantly reduced.

## Impact

- With a small `shrink.count` and high concurrency, the shrinker may produce a
  less minimal failing value than the synchronous checker.
- The asynchronous test `synchronous_and_asynchronous_produce_same_results`
  already guards against this by using `concurrency = 1` for the comparison.
- There is no correctness issue (the checker always terminates and always
  returns a genuinely failing value), but the *minimality* of the result may
  differ from the synchronous path when concurrency is high.

## Testing Recommendations

1. Add a test that verifies: with concurrency > 1 and a small shrink budget
   (e.g. `shrink.count = 4`), the async checker still finds the minimum failing
   value for a simple predicate (e.g. `value < 50` on `0u8..=100`).
2. Add a test exercising I/O-bound async properties (e.g. a check that awaits
   `tokio::time::sleep` or `futures_lite::future::yield_now` multiple times)
   to confirm that the stream is correctly woken up and continues to make
   progress after pending futures resolve.
3. Add a test with high concurrency on a property with many failing values to
   confirm that no deadlock or starvation occurs.

## Related

- `checkito/src/check.rs:988-994` — buffer reset on new failure
- `checkito/tests/asynchronous.rs:93-128` — determinism test (uses concurrency=1)
- `checkito/tests/asynchronous.rs:122-144` — concurrency parameter test
