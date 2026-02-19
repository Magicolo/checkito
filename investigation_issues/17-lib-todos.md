# Additional TODOs in lib.rs - Missing Features and API Improvements

## Summary
The main library file (`lib.rs`) contains 4 TODO comments describing missing features and API improvements that should be addressed.

## Context
Located at `checkito/src/lib.rs:130-136`, these TODOs represent architectural improvements and missing features identified by the library author.

## The TODOs

### TODO 1: Asynchronous Checks Hang Forever (CRITICAL)
**Location**: Line 132

```rust
// TODO:
// - Asynchronous checks seem to hang forever. Add tests.
```

**Issue**: The async functionality appears to have bugs causing hangs.

**Current State**:
- `#[check(..., asynchronous = true)]` is supported (macro attribute)
- But actual async execution may hang indefinitely
- **NO TESTS** for async functionality

**Impact**:
- Users cannot reliably use async property tests
- Silent hangs are difficult to debug
- May cause CI/CD pipelines to timeout

**Recommended Actions**:
1. **Reproduce the hang**:
   ```rust
   #[check(0..10, asynchronous = true)]
   async fn test_async(x: i32) {
       tokio::time::sleep(Duration::from_millis(10)).await;
       assert!(x < 100);
   }
   // Does this hang? Under what conditions?
   ```

2. **Add async tests** in `tests/asynchronous.rs`:
   - Basic async check
   - Async with futures
   - Async with tokio::spawn
   - Async error handling
   - Async panic handling
   - Timeout behavior

3. **Investigate**:
   - Is it a runtime issue (tokio vs async-std)?
   - Is it a polling issue?
   - Is it related to shrinking in async context?
   - Check `check.rs` async implementation (lines 723-850)

4. **Fix**:
   - Add timeout mechanism
   - Proper async runtime handling
   - Clear error messages if async hangs

**Related Code**:
- `checkito/src/check.rs`: Lines 723-850 (async Checker impl)
- `tests/asynchronous.rs`: Has some tests but may not cover hang cases

**Priority**: **CRITICAL** - Hanging tests are unacceptable

---

### TODO 2: Adaptive Check Count Based on Runtime
**Location**: Line 133

```rust
// - Instead of running a fixed number of checks, determine the number of checks based on the runtime of the generation and check.
```

**Issue**: Currently uses fixed `GENERATES = 1024` checks, regardless of test complexity.

**Current Limitation**:
- Fast tests: waste time running 1000+ unnecessary iterations
- Slow tests: might need fewer iterations to stay within reasonable runtime

**Proposed Feature**: Adaptive testing
```rust
// Pseudo-code
let target_duration = Duration::from_secs(5);
let mut checks = 0;
let start = Instant::now();

while start.elapsed() < target_duration {
    run_check();
    checks += 1;
    
    // Estimate remaining iterations based on avg time per check
    if checks >= minimum_checks {
        break;
    }
}
```

**Benefits**:
- Consistent test runtime across fast/slow properties
- Better resource utilization
- Configurable timeout instead of fixed count

**Implementation**:
1. Add `Generates::duration` option (alternative to `count`)
2. Track time per check
3. Estimate total checks based on target duration
4. Fall back to `count` if specified

**API Design**:
```rust
pub struct Generates {
    pub count: usize,           // Existing
    pub duration: Option<Duration>,  // New! Adaptive mode
    pub minimum: usize,         // Minimum checks even if duration exceeded
}
```

**Priority**: Medium - Nice to have, not critical

---

### TODO 3: Support for Parallel Checks
**Location**: Line 134

```rust
// - Support for 'parallel' checks.
```

**Issue**: Checks run sequentially, but could run in parallel for speed.

**Current State**:
- `parallel.rs` module exists for parallel generation
- But checking itself is not parallelized

**Proposed Feature**: Parallel property testing
```rust
#[check(0..100, parallel = true, workers = 4)]
fn test_concurrent(x: i32) {
    // Runs on multiple threads simultaneously
    assert!(x < 100);
}
```

**Benefits**:
- Faster test execution (use all CPU cores)
- Find concurrency bugs earlier
- Reduce CI time

**Challenges**:
- Thread safety of generators
- Reproducibility (need deterministic scheduling)
- Shrinking in parallel (complex!)
- Output formatting (interleaved results)

**Related Code**:
- `checkito/src/parallel.rs`: Already has parallel infrastructure
- Could extend for parallel checking

**Implementation Strategy**:
1. Reuse `parallel::Pool` from parallel.rs
2. Partition check count across workers
3. Each worker runs subset of checks
4. Aggregate results at the end
5. Shrinking runs on single thread (simpler)

**Priority**: Medium - Performance optimization, not a bug

---

### TODO 4: Review Public API and Make Things Private
**Location**: Lines 135-136

```rust
// - Review public api and make things more private to prevent breaking changes; especially modules.
```

**Issue**: Too many things are public, making it hard to evolve the API without breaking changes.

**Current State**:
```rust
pub mod all;
pub mod any;
pub mod array;
pub mod boxed;
pub mod cardinality;
pub mod check;
// ... many public modules
```

**Recommendation**: API audit
1. **Keep Public** (core API):
   - `Generate`, `Shrink`, `Prove`, `Check` traits
   - `check` macro
   - Common generators from prelude

2. **Make Private** (implementation details):
   - Internal modules: `utility`, `run`
   - Specific implementation types (unless users need them)
   - Helper functions

3. **Re-export Selectively**:
   ```rust
   pub mod generate {
       // Public trait
       pub use crate::generate::{Generate, FullGenerate};
   }
   
   // Implementation details not exposed
   mod filter { /* ... */ }
   mod map { /* ... */ }
   ```

**Benefits**:
- Smaller public API surface
- Easier to evolve internals
- Less risk of breaking changes
- Better SemVer compliance

**Migration Strategy**:
1. Audit current public items
2. Categorize: must-be-public vs can-be-private
3. Use `#[doc(hidden)]` for transition period
4. Add deprecation warnings
5. Make private in next major version

**Priority**: Medium - Important for long-term maintainability

---

## Summary Table

| TODO | Priority | Effort | Type |
|------|----------|--------|------|
| Async hangs | **CRITICAL** | Medium | Bug |
| Adaptive count | Medium | Medium | Feature |
| Parallel checks | Medium | Large | Feature |
| API review | Medium | Large | Refactor |

## Acceptance Criteria

### For Async Hangs (CRITICAL)
- [ ] Reproduce the hang
- [ ] Add comprehensive async tests
- [ ] Fix the hang or add clear timeout
- [ ] Document async limitations

### For Adaptive Count
- [ ] Design duration-based API
- [ ] Implement time tracking
- [ ] Add tests for adaptive behavior
- [ ] Document usage

### For Parallel Checks
- [ ] Prototype parallel checking
- [ ] Ensure reproducibility
- [ ] Handle shrinking correctly
- [ ] Add parallel tests

### For API Review
- [ ] Audit all public items
- [ ] Propose private/public split
- [ ] Deprecation plan
- [ ] SemVer compliance

## Related Issues
- Issue #3: Missing test coverage (related to async tests)
- Issue #12: Unsafe parallel code (related to parallel checks)
- General API design and evolution

## Priority Assessment
**Async hangs**: **CRITICAL** - Must fix  
**Others**: Medium - Enhancements, not urgent
