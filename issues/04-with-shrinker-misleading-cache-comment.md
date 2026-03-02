# `with::Shrinker::item()` Misleading Comment: Cache Is Not Persistent

## Summary

The documentation comment on `with::Shrinker` (in `checkito/src/standard.rs`) claims that
the `RefCell<Option<T>>` cache "ensures that multiple calls to `item()` return the same
value", but the implementation immediately **takes** (removes) the cached value so that
every subsequent call to `item()` re-invokes the closure.  The comment is therefore
misleading, and there is a subtle behavioral inconsistency between the documentation of the
`Shrink` trait (which implies stable `item()` return values) and the actual implementation.

## Affected Code

`checkito/src/standard.rs` – `with::Shrinker` and its `Shrink` implementation:

```rust
/// Shrinker that caches the generated value to ensure consistent `item()`
/// calls.
///
/// The `RefCell<Option<T>>` is used to cache the generated value on first
/// access. This ensures that multiple calls to `item()` return the same
/// value, which is required by the `Shrink` trait contract.
///
/// The cached value is `None` initially and populated on the first `item()`
/// call, then consumed (taken) and returned. Subsequent calls will
/// regenerate the value.
pub struct Shrinker<T, F> {
    generator: F,
    cached: RefCell<Option<T>>,
}
```

```rust
fn item(&self) -> Self::Item {
    let mut cached = self.cached.borrow_mut();
    if cached.is_none() {
        *cached = Some((self.generator)());
    }
    cached
        .take()                             // <-- removes the value!
        .expect("cached value must be Some after initialization")
}
```

Note the **contradiction** in the doc comment:

1. First sentence: *"This ensures that multiple calls to `item()` return the same value"*
2. Later: *"then consumed (taken) and returned. Subsequent calls will regenerate the value."*

## Why This Matters

### Trait Contract

The `Shrink` trait documentation says:

> A shrinker is essentially a lazy iterator over simpler versions of a value.

While `Shrink` does not explicitly say `item()` must be idempotent, the practical use of the
trait (e.g., in the synchronous and asynchronous iterators in `check.rs`) calls `item()` more
than once on the same shrinker state.  For example, a shrunk value is recorded in a
`Result::Shrunk` or `Result::Fail` by calling `old_shrinker.item()` and the new shrinker's
`item()` independently.  If `item()` is non-deterministic, the reported items could differ
between calls.

### Practical Consequence

The `With<F>` generator explicitly documents:

> **This generator assumes that the closure always returns the same value; if this assumption
> is violated, `check`ing and `shrink`ing may have unexpected behaviors.**

So the expectation is that `F` always returns the same value.  Under this assumption, calling
the closure multiple times is correct.  However the **comment** in `Shrinker` saying "ensures
multiple calls return the same value" is false — no caching actually persists across calls.

If a user supplies a closure that returns different values on successive calls (which the API
explicitly warns against), the mismatch between expectation and reality is harder to diagnose
because the incorrect comment implies a cache-based safeguard exists when it does not.

## Proposed Fix

### Option A – Actually cache the value (require `T: Clone`)

If `T: Clone`, store the value permanently and return a clone on each call:

```rust
fn item(&self) -> Self::Item {
    let mut cached = self.cached.borrow_mut();
    if cached.is_none() {
        *cached = Some((self.generator)());
    }
    cached.as_ref().unwrap().clone()
}
```

This would require adding a `T: Clone` bound to `Shrinker<T, F>` (or to the `Shrink` impl).
Since `Shrink` itself requires `Clone` on the shrinker (not necessarily on `T`), this is an
additional constraint that would limit usability.

### Option B – Remove the misleading "caching" claim from the comment (minimal fix)

Update the documentation to accurately describe what happens — the closure is called once
per `item()` invocation:

```rust
/// Shrinker produced by [`With`].
///
/// Because [`item()`](Shrink::item) must return an owned `T` from a shared
/// reference (`&self`), the closure is re-invoked on every call.  The
/// [`With`](super::with::With) generator therefore requires its closure to
/// always return the same value; violating this contract may lead to
/// inconsistent test-case reports.
pub struct Shrinker<T, F> { ... }
```

And simplify `item()`:

```rust
fn item(&self) -> Self::Item {
    (self.generator)()
}
```

This removes the `RefCell<Option<T>>` field entirely, making the code smaller and clearer.

### Option C – Cache permanently without Clone (via `UnsafeCell` / `take`)

Replace `RefCell<Option<T>>` with a design that keeps the value alive without requiring
`Clone`, by not taking it.  This requires returning `&T` from `item()`, but the `Shrink`
trait returns `T` by value — so this path is not directly compatible without a trait change.

### Recommended Approach

**Option B** is the minimal fix:
- Remove the `RefCell<Option<T>>` field.
- Call the closure directly in `item()`.
- Fix the misleading documentation.
- Add a comment explaining why the closure is called on every `item()` invocation.

The `Clone` approach (Option A) adds a constraint that wasn't there before and is not
strictly necessary.

## Secondary Issue: `Clone` Implementation Clears the Cache

The `Clone` impl for `Shrinker<T, F>` currently clears the cache:

```rust
impl<T, F: Clone> Clone for Shrinker<T, F> {
    fn clone(&self) -> Self {
        Self {
            generator: self.generator.clone(),
            cached: RefCell::new(None),   // always starts empty
        }
    }
}
```

This means that if a value was already cached before cloning, the clone does not inherit it.
This is another subtle inconsistency: `item()` on a cloned shrinker always re-invokes the
closure, even if the original had a cached value.

With Option B (remove the cache), this implementation also becomes trivially correct.

## Test Cases

Once the fix is applied:

```rust
#[test]
fn with_shrinker_item_is_consistent() {
    use checkito::prelude::*;
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let c = counter.clone();
    let gen = with(move || {
        let n = c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        n
    });
    let mut state = checkito::state::State::random(0, 1, Default::default(), 0);
    let shrinker = gen.generate(&mut state);
    // item() should be consistent (always returns the same value per the contract).
    let _ = shrinker.item();
    let _ = shrinker.item();
    // With the fix (Option B), the closure is called each time; behavior is
    // well-defined as long as the closure is deterministic.
}
```
