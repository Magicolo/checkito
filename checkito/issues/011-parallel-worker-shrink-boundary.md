# Issue: parallel worker shrink boundary check appears off-by-one and may keep extra workers alive

## Summary
In `parallel::State::run`, worker shutdown on pool shrink uses:

```rust
if state.size.end.load(Ordering::Relaxed) < index {
    state.size.start.fetch_sub(1, Ordering::Relaxed);
    break;
}
```

Given worker indices are compared against `end` bounds, this `< index` condition appears to permit a worker whose `index == end` to continue running, which is likely outside the intended active range.

This looks related to index-boundary handling in `State::next` and could cause worker accounting mismatches when the pool size is reduced.

## Where this happens
- `checkito/src/parallel.rs`
  - Worker allocation / index progression in `State::next`.
  - Shrink handling condition in `State::run` (`end < index`).

## Why this is an issue
1. **Potential extra worker retention after shrink**: boundary worker may not exit when it should.
2. **Accounting drift risk**: `size.start` and `ready` counters are subtle; keeping a stale worker can complicate invariants.
3. **Hard-to-reproduce concurrency behavior**: bugs may only surface under dynamic resizing and load.
4. **Related known boundary concerns**: there is already a worker-count off-by-one concern in this module; this path deserves focused verification.

## Investigation notes to include in implementation work
- Clarify intended index domain (`0..end`, `1..=end`, etc.) and document it in code comments.
- Validate consistency among:
  - `State::next` compare/exchange logic,
  - `State::run` shrink exit condition,
  - `Pool::with` size changes,
  - `ready`/`size.start` updates.

## Proposed fix plan
1. Write targeted tests (likely stress/integration style) for dynamic pool shrink behavior:
   - start with larger pool,
   - shrink while tasks are active,
   - verify expected active worker count and progress completion.
2. Verify boundary condition; likely `<=` (or adjusted indexing model) is needed.
3. Audit `fetch_sub` usage for underflow assumptions and ensure counters remain valid.
4. Add inline comments explaining index convention and shrink behavior.

## Acceptance criteria
- Deterministic tests confirm no extra workers remain after shrink.
- No hangs/starvation under repeated grow/shrink cycles.
- Worker-count semantics are documented and internally consistent.
