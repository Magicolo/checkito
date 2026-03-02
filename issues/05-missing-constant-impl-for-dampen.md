# Missing `Constant` Implementation for `Dampen<G>`

## Summary

The `Dampen<G>` wrapper in `checkito/src/dampen.rs` does not implement the `Constant` trait,
unlike every other wrapping generator in the library.  This means `Dampen` cannot be used in
const contexts with the `constant!` macro, and the `Constant::VALUE` pattern that allows
static exhaustive-mode optimization is unavailable for dampened generators.

## Context

The `Constant` trait is defined in `checkito/src/primitive.rs`:

```rust
pub trait Constant {
    const VALUE: Self;
}
```

It is implemented for all wrapping generators in the library so that they can be constructed
as compile-time constants.  Examples from the codebase:

| Type | Constant impl |
|------|---------------|
| `Any<C>` | `Any<C>(C::VALUE)` |
| `Keep<C>` | `Keep<C>(C::VALUE)` |
| `Unify<C, I>` | `Unify(PhantomData, C::VALUE)` |
| `Flatten<C>` | `Flatten(C::VALUE)` |
| `Array<C, N>` | `Array(C::VALUE)` |
| `Convert<C, I>` | `Convert(PhantomData, C::VALUE)` |
| `Collect<I, C, F>` | struct literal with `C::VALUE` and `I::VALUE` |
| `Cardinality<G, C>` | `Cardinality(G::VALUE)` |
| `Dampen<G>` | **not implemented** |

## Affected Code

`checkito/src/dampen.rs` – the entire file contains only the struct definition and one
`Generate` implementation.  The `Constant` impl is absent.

```rust
// Current file (no Constant impl):
use crate::{generate::Generate, state::State};

#[derive(Clone, Debug)]
pub struct Dampen<G: ?Sized> {
    pub(crate) pressure: f64,
    pub(crate) deepest: usize,
    pub(crate) limit: usize,
    pub(crate) generator: G,
}
```

## Why This Is a Problem

### 1. Breaks the Pattern Established by All Other Wrappers

Every wrapper that contains a `G: Constant` inner generator provides `Constant for Wrapper<G>`
so that composite generators can be composed statically.  `Dampen` is the odd one out.

### 2. `constant!` Macro Cannot Produce a `Dampen`

The `constant!` proc-macro in `checkito_macro` converts expressions into statically-typed
generator values.  If a user writes:

```rust
constant!(some_generator.dampen())
```

the macro cannot produce a properly typed result because `Dampen` has no `Constant::VALUE`.

### 3. Prevents Full Static Cardinality Tracking

When `Constant` is implemented, the library can determine the exact generator configuration
at compile time, enabling fully static cardinality computation and potentially reducing
binary size.

## Proposed Fix

Add the following implementation to `checkito/src/dampen.rs`:

```rust
use crate::primitive::Constant;

impl<G: Constant> Constant for Dampen<G> {
    const VALUE: Self = Self {
        pressure: 0.0,
        deepest: 0,
        limit: 0,
        generator: G::VALUE,
    };
}
```

### Choice of Default Values for `pressure`, `deepest`, and `limit`

The semantics of `Constant::VALUE` for a wrapper are "the canonical zero/identity
configuration of the generator."  For `Dampen`:

- `pressure: 0.0` — no pressure applied.
- `deepest: 0` — trigger dampening at depth 0 (i.e., immediately).
- `limit: 0` — trigger dampening at limit 0 (i.e., immediately).

Both `deepest = 0` and `limit = 0` mean the `dampen` function in `State` always sets the
size to `0.0` (since `depth >= 0` and `limit >= 0` are always true).  This is a conservative
default: when a `Dampen` is used as a `Constant` (e.g., inside `constant!()`), generation
always produces minimal values.

An alternative is to use values that pass through unchanged:

- `pressure: 1.0`, `deepest: usize::MAX`, `limit: usize::MAX`

This would make `Constant::VALUE` behave more like "no dampening" rather than "always
dampen."  The choice should be documented clearly.

### Recommended Approach

Use `deepest: usize::MAX` and `limit: usize::MAX` to make `Constant::VALUE` a pass-through
configuration (no dampening by default), as this is more useful in practice:

```rust
impl<G: Constant> Constant for Dampen<G> {
    const VALUE: Self = Self {
        pressure: 1.0,
        deepest: usize::MAX,
        limit: usize::MAX,
        generator: G::VALUE,
    };
}
```

This matches the expectation that `Constant::VALUE` for a wrapper is its "most general" form.

## Test Cases to Add

In `checkito/tests/constant.rs`:

```rust
#[test]
fn dampen_has_constant_value() {
    use checkito::{dampen, primitive::Constant, primitive::i32::I32};
    let _ = <checkito::dampen::Dampen<I32<0>>>::VALUE;
    // Ensure the constant is well-typed and accessible.
}

#[test]
fn constant_macro_works_with_dampen() {
    use checkito::*;
    let gen = constant!(0i32).dampen();
    // Should compile and function correctly.
    let _ = gen.samples(10).collect::<Vec<_>>();
}
```
