# `check.rs` Synchronous Iterator Calls `shrinker.item()` Twice

## Summary

In the synchronous `Iterator::next()` implementation in `check.rs`, the `shrinker.item()`
method is called **twice** in the pass branch: once to check the property and once to
produce the `Pass` result.  This violates the contract of generators like `with::Shrinker`,
which regenerate values on every `item()` call (the cache is consumed and refilled each
time).

## Affected Code

`checkito/src/check.rs` – `impl iter::Iterator for synchronous::Iterator`:

```rust
Machine::Generate { generator, mut states, shrinks } => {
    let mut state = states.next()?;
    let shrinker = generator.generate(&mut state);
    match handle(shrinker.item(), &mut self.check) {   // ← first call to item()
        Ok(proof) => {
            self.machine = Machine::Generate { … };
            if self.yields.passes {
                break Some(pass(shrinker.item(), state, proof));  // ← second call to item()
            }
        }
        …
    }
}
```

The issue is that `handle(shrinker.item(), …)` and `pass(shrinker.item(), …)` call `item()`
on the same `shrinker` instance twice.

## Concrete Impact

### `with::Shrinker`

`with::Shrinker` stores the generated value in a `RefCell<Option<T>>` that is **consumed
(taken)** on each call to `item()`:

```rust
fn item(&self) -> Self::Item {
    let mut cached = self.cached.borrow_mut();
    if cached.is_none() {
        *cached = Some((self.generator)());  // generate value
    }
    cached.take()                           // take it out!
        .expect("…")
}
```

This means:
1. First `item()` call: generates value `v1`, takes it from cache → returns `v1`.
2. Second `item()` call: cache is `None`, calls closure again → returns `v2`.

If the closure is deterministic, `v1 == v2` and everything is fine.  However:
- The documented assumption is that the closure "always returns the same value."
- The double-call is an implicit violation of an API that was designed to support the single-call
  pattern.
- Any future generator that produces different values on successive `item()` calls would
  silently break the test-reporting logic (the property is evaluated with `v1` but the
  pass/fail is reported with `v2`).

## Similar Patterns

The same double-call pattern appears in several places in the synchronous and asynchronous
implementations:

1. **Synchronous `Machine::Generate` (pass branch):** `handle(shrinker.item(), …)` then
   `pass(shrinker.item(), …)` — as shown above.
2. **Synchronous `Machine::Shrink` (shrink branch, ok case):** `handle(new_shrinker.item(), …)` then
   `shrink(new_shrinker.item(), …)`.
3. **Synchronous `Machine::Shrink` (shrink branch, err case):** `old_shrinker.item()` appears in
   the `handle` call (via `new_shrinker.item()` indirectly) and then in `shrunk(old_shrinker.item(), …)`.

## Fix Plan

### Option A – Cache the `item()` result before use

Introduce a local binding:

```rust
Machine::Generate { generator, mut states, shrinks } => {
    let mut state = states.next()?;
    let shrinker = generator.generate(&mut state);
    let item = shrinker.item();                      // ← call once
    match handle(item.clone(), &mut self.check) {    // ← use clone for handle
        Ok(proof) => {
            self.machine = Machine::Generate { … };
            if self.yields.passes {
                break Some(pass(item, state, proof)); // ← use original
            }
        }
        …
    }
}
```

This requires `G::Item: Clone` to pass one copy to `handle` and keep another for reporting.

### Option B – Restructure `handle` to take the item by reference

Change `handle` to accept `&T` and clone internally when needed, so the original owned
value is preserved for the pass report.

### Option C – Store `item()` in the `Shrinker` type itself (trait change)

Add an `item_ref(&self) -> &Self::Item` method to `Shrink` alongside `item(&self) ->
Self::Item`.  This would allow the check engine to inspect the item without consuming it.
This is a more invasive change.

### Recommended

**Option A** is the minimal fix.  The `G::Item: Clone` bound is already required by many
parts of the library (e.g., `Shrinker::clone()`) and is likely satisfied in practice.

However, the `Check` trait's `check` method has `G::Item` without a `Clone` bound:

```rust
pub trait Check: Generate {
    fn check<P: Prove, C: FnMut(Self::Item) -> P>(
        &self,
        check: C,
    ) -> Option<Fail<Self::Item, P::Error>>
    where
        Self::Item: Debug;
}
```

Adding `G::Item: Clone` here would be a public API change.  For minimal impact, the fix
could be applied only to the internal `Iterator::next()` by cloning inside the implementation
rather than adding it to the public trait bound.

## Relationship to `with::Shrinker` Issue

This issue is related to the `with::Shrinker` caching issue (see issue `04-with-shrinker-
misleading-cache-comment.md`).  The simplest combined fix is:

1. Make `with::Shrinker::item()` call the closure once and store the result permanently
   (removing the `take()` behavior).
2. Keep the `item()` double-call in the check engine as-is.

But this approach only partially resolves the issue — other generators that have non-idempotent
`item()` calls would still be affected.

## Test Cases to Add

```rust
#[test]
fn item_is_called_only_once_per_generation_step() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    let generator = with(move || {
        c.fetch_add(1, Ordering::SeqCst);
        42u8
    });

    // Run a single check with items=true to trigger the pass reporting path.
    let mut checker = generator.checker();
    checker.generate.count = 1;
    checker.generate.items = true;
    let results: Vec<_> = checker.checks(|_| true).collect();

    assert_eq!(results.len(), 1);
    // The item should have been generated once per check step, not twice.
    assert_eq!(count.load(Ordering::SeqCst), 1,
        "item() was called more than once for a single generation step");
}
```
