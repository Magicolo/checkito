# Issue: parallel pool worker index allocation likely under-spawns workers (off-by-one)

## Summary
`checkito/src/parallel.rs` appears to allocate worker indices in a way that may spawn fewer threads than requested. The allocator returns `Some(next)` only when `next < end`, with `start` initialized to `0` and `end` set to configured pool size.

This implies the spawned index range is effectively `1..end`, i.e., `end - 1` workers, and can yield zero workers for `end = 1`.

## Why this is a problem
- Reduced parallel throughput relative to requested parallelism.
- Edge-case risk of starvation/hang when minimal pool sizes are used.
- Internal invariants for `ready` accounting may become difficult to reason about.

## Evidence and context
- `State::new` initializes range as `start = 0`, `end = size(...)`.
- `State::next` computes `next = start + 1` and returns `Some(next)` only when `next < end`.
- `State::ensure` repeatedly calls `next()` to decide spawning.
- `State::run` includes a shrink check comparing `end < index`, which also depends on index semantics.

## Scope
- Primary file: `checkito/src/parallel.rs`.
- Potentially affected tests or codepaths using `parallel::iterate` and pool resizing.

## Proposed fix plan
1. **Add tests first:**
   - Unit/integration tests for exact spawned-worker count across small sizes (`1`, `2`, `3`).
   - Verify no blocking/hang for minimal configured pool + simple producer closure.
2. **Normalize index semantics:**
   - Adopt either zero-based `[0, end)` or one-based `[1, end]` consistently.
   - Update `State::next` and `State::run` shrink checks accordingly.
3. **Revalidate accounting:**
   - Ensure `ready.fetch_add/sub` transitions still match lifecycle.
4. **Stress validation:**
   - Run repeated short parallel iterations to check deterministic completion.

## Risks and caveats
- Thread-count changes can alter timing-sensitive tests.
- Panic propagation path (`error.try_send`, resume unwind) should be rechecked after index updates.

## Acceptance criteria
- Observed active worker count matches requested count for representative sizes.
- No deadlocks/hangs in low-parallelism scenarios.
- Existing parallel tests pass with new targeted regression tests.
