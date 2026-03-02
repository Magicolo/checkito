# `boxed.rs` Uses `unwrap()` for Internal Downcasts That Should Use `expect()`

## Summary

The internal helper functions in `checkito/src/boxed.rs` use `.unwrap()` on type downcasts
that are guaranteed to succeed by construction.  Per the project's engineering standards
("Use `expect()` with descriptive invariant violation messages instead of `unwrap()` for type
downcasts and internal invariants"), these should use `.expect()` with a clear message that
explains *why* the invariant holds and what it means if it is violated.

## Affected Code

`checkito/src/boxed.rs` – five helper functions at the bottom of the file:

```rust
fn generate<G: Generate + 'static>(generator: &dyn Any, state: &mut State) -> Shrinker<G::Item>
where
    G::Shrink: 'static,
{
    Shrinker::new(Box::new(
        generator.downcast_ref::<G>().unwrap().generate(state),  // line 124
    ))
}

fn cardinality<G: Generate + 'static>(generator: &dyn Any) -> Option<u128> {
    generator.downcast_ref::<G>().unwrap().cardinality()  // line 129
}

fn clone<S: Shrink + 'static>(shrinker: &dyn Any) -> Box<dyn Any> {
    Box::new(shrinker.downcast_ref::<S>().unwrap().clone())  // line 133
}

fn item<S: Shrink + 'static>(shrinker: &dyn Any) -> S::Item {
    shrinker.downcast_ref::<S>().unwrap().item()  // line 137
}

fn shrink<S: Shrink + 'static>(shrinker: &mut dyn Any) -> Option<Box<dyn Any>> {
    Some(Box::new(shrinker.downcast_mut::<S>().unwrap().shrink()?))  // line 141
}
```

## Why the Invariant Holds

In `Boxed<I>`, the constructor stores a `Box<G>` as a `Box<dyn Any>` and captures the
concrete type `G` in the function pointers (`generate: fn(&dyn Any, &mut State) -> ...`).
The `generate` function pointer is specialized for `G`, so when it is called, the `&dyn Any`
argument is guaranteed to hold a `G`.  The downcast therefore cannot fail as long as no
unsafe code violates the invariant.

The same reasoning applies to the `Shrinker<I>` helpers: the function pointers stored in
`Shrinker<I>` are specialized for the concrete shrinker type `S`, so the `dyn Any` argument
is always an `S`.

## Why `.expect()` is Preferred

Using `.expect()` with a descriptive message:
1. Communicates the invariant to future readers of the code.
2. Produces a more useful panic message if the invariant is ever violated (e.g., by a bug in
   unsafe code or future refactoring).
3. Aligns with the coding standard established elsewhere in the codebase (e.g.,
   `standard.rs:331` already uses `.expect("cached value must be Some after initialization")`).

## Proposed Fix

Replace each `.unwrap()` with `.expect(...)`:

```rust
fn generate<G: Generate + 'static>(generator: &dyn Any, state: &mut State) -> Shrinker<G::Item>
where
    G::Shrink: 'static,
{
    Shrinker::new(Box::new(
        generator
            .downcast_ref::<G>()
            .expect("boxed generator type mismatch: type erasure invariant violated")
            .generate(state),
    ))
}

fn cardinality<G: Generate + 'static>(generator: &dyn Any) -> Option<u128> {
    generator
        .downcast_ref::<G>()
        .expect("boxed generator type mismatch: type erasure invariant violated")
        .cardinality()
}

fn clone<S: Shrink + 'static>(shrinker: &dyn Any) -> Box<dyn Any> {
    Box::new(
        shrinker
            .downcast_ref::<S>()
            .expect("boxed shrinker type mismatch: type erasure invariant violated")
            .clone(),
    )
}

fn item<S: Shrink + 'static>(shrinker: &dyn Any) -> S::Item {
    shrinker
        .downcast_ref::<S>()
        .expect("boxed shrinker type mismatch: type erasure invariant violated")
        .item()
}

fn shrink<S: Shrink + 'static>(shrinker: &mut dyn Any) -> Option<Box<dyn Any>> {
    Some(Box::new(
        shrinker
            .downcast_mut::<S>()
            .expect("boxed shrinker type mismatch: type erasure invariant violated")
            .shrink()?,
    ))
}
```

## Scope

This is a pure code-quality change with no behavioral impact.  No test changes are needed,
but it is worth reviewing whether the existing test for `Boxed` in the test suite provides
adequate coverage of the downcast path.
