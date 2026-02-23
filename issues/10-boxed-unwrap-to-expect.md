# Issue: unwrap() Used for Internal Type Downcasts in boxed.rs

## Summary

Several private helper functions in `checkito/src/boxed.rs` use `.unwrap()`
when downcasting type-erased `dyn Any` values. These calls should succeed by
construction (the types are erased and restored within the same module), but
if an invariant is violated the panic message will be uninformative. Using
`.expect("...")` with a descriptive message would make debugging significantly
easier.

## Location

- `checkito/src/boxed.rs:124` — `generator.downcast_ref::<G>().unwrap()`
- `checkito/src/boxed.rs:129` — `generator.downcast_ref::<G>().unwrap()`
- `checkito/src/boxed.rs:133` — `shrinker.downcast_ref::<S>().unwrap()`
- `checkito/src/boxed.rs:137` — `shrinker.downcast_ref::<S>().unwrap()`
- `checkito/src/boxed.rs:141` — `shrinker.downcast_mut::<S>().unwrap()`

## Code

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

## Why These Are Invariants

These downcasts are type-safe by construction: the function pointers stored in
the `Boxed<I>` struct are created at the same time as the `Box<dyn Any>` they
operate on, so the type `G` / `S` is always correct. The `unwrap()` should
never fire. However, `.expect("...")` with a clear description of the violated
invariant is a better practice for this class of internal downcast.

Compare with `standard.rs:338` which already uses:
```rust
.expect("cached value must be Some after initialization")
```

## Proposed Fix

Replace each `.unwrap()` with `.expect("<invariant description>")`:

```rust
generator
    .downcast_ref::<G>()
    .expect("generator type must match the concrete type G stored in the Boxed<I>")
    .generate(state)
```

```rust
shrinker
    .downcast_ref::<S>()
    .expect("shrinker type must match the concrete type S stored in the Boxed<I>")
    .item()
```

etc.

## Impact

Low risk / low effort. The change is purely cosmetic and has no runtime effect
under normal operation. It improves debuggability for maintainers.

## Related

- `checkito/src/standard.rs:338` (uses `expect` — good example)
- Project memory: "Use expect() with descriptive invariant violation messages
  instead of unwrap() for type downcasts and internal invariants"
