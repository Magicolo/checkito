# Missing `FullGenerate` for `NonZero*` Integer Types

## Summary

The library provides `FullGenerate` (a "default generator" trait) for all primitive integer
types (`u8`, `u16`, …, `i128`, `usize`, `isize`), but not for the corresponding `NonZero*`
types from `core::num` (`NonZeroU8`, `NonZeroU16`, …, `NonZeroI8`, …, `NonZeroUsize`,
`NonZeroIsize`).  These types are frequently used in real-world Rust code and should be
directly usable with the `_` / `..` inference syntax in `#[check]` attributes and with
`FullGenerate::generator()`.

## Context

`FullGenerate` is defined as:

```rust
pub trait FullGenerate {
    type Item;
    type Generator: Generate<Item = Self::Item>;
    fn generator() -> Self::Generator;
}
```

It is implemented for all 12 integer primitives, `bool`, `char`, `String`, all standard
collections, `Option<G>`, `Result<T, E>`, smart pointers, and arrays.

Notably absent: **all `NonZero*` types**.

## Impact

```rust
// Does NOT compile today:
#[check(_)]
fn nonzero_is_nonzero(n: NonZeroU8) {
    assert!(n.get() > 0);
}

// Must use a manual workaround:
#[check(Generate::filter_map(1u8..=255, NonZeroU8::new))]
fn nonzero_is_nonzero_workaround(n: Option<NonZeroU8>) {
    assert!(n.unwrap().get() > 0);
}
```

The workaround is verbose and produces `Option<NonZeroU8>` instead of `NonZeroU8`, requiring
extra unwrapping that adds noise to the test.

## Proposed Fix

Add `FullGenerate` implementations for all 12 `NonZero*` types.  The simplest correct
approach uses a `Range` that excludes zero:

```rust
use core::num::{
    NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
    NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize,
};
use crate::{generate::{FullGenerate, Generate}, map::Map, primitive::Range};

macro_rules! nonzero_generate {
    ($nonzero:ident, $inner:ident) => {
        impl FullGenerate for $nonzero {
            type Item = $nonzero;
            type Generator = Map<
                Range<$inner>,
                fn($inner) -> $nonzero,
            >;

            fn generator() -> Self::Generator {
                // Generate the inner type, excluding 0.
                Range($inner::MIN + (1 as $inner), $inner::MAX)
                    // Actually need to handle signed types carefully:
                    // for signed types, exclude only 0, not the MIN.
                    .map($nonzero::new_unchecked as fn($inner) -> $nonzero)
            }
        }
    };
}
```

There are a few design choices:

### Option A – Generate the full non-zero range

For unsigned types (`NonZeroU8`), the generator covers `1..=255`.
For signed types (`NonZeroI8`), the generator covers `-128..=-1` and `1..=127`.

The cleanest implementation uses `any` to merge the negative and positive ranges for signed
types:

```rust
impl FullGenerate for NonZeroI8 {
    type Generator = impl Generate<Item = NonZeroI8>;

    fn generator() -> Self::Generator {
        (i8::MIN..=-1i8, 1i8..=i8::MAX)
            .any()
            .unify::<i8>()
            .map(|n| NonZeroI8::new(n).expect("range excludes zero"))
    }
}
```

### Option B – Use `filter_map` (simpler but less efficient)

```rust
impl FullGenerate for NonZeroU8 {
    type Generator = FilterMap<Range<u8>, fn(u8) -> Option<NonZeroU8>>;

    fn generator() -> Self::Generator {
        Generate::filter_map(u8::generator(), NonZeroU8::new)
    }
}
```

This produces `Option<NonZeroU8>` internally but the filter step handles `None` transparently.
However, the `Item = Option<NonZeroU8>` for `Filter` means this approach requires a `map` to
unwrap safely.

### Option C – Direct range excluding zero (simplest)

For unsigned types:

```rust
impl FullGenerate for NonZeroU8 {
    type Item = NonZeroU8;
    type Generator = Map<ops::RangeInclusive<u8>, fn(u8) -> NonZeroU8>;

    fn generator() -> Self::Generator {
        (1u8..=u8::MAX).map(|n| {
            // SAFETY: n is guaranteed > 0 by the range.
            unsafe { NonZeroU8::new_unchecked(n) }
        })
    }
}
```

For signed types, this cannot be done with a single contiguous range.

### Recommended

Use **Option C** for unsigned and a two-range `any().unify()` approach for signed types.
Wrap in a helper macro to reduce boilerplate.

## Location for Implementation

Add to `checkito/src/standard.rs` in a new `pub mod nonzero { ... }` section, mirroring the
pattern of `pub mod number`, `pub mod character`, etc.

Also add `FullGenerate` for the types in the module root.

## Test Cases to Add

```rust
#[test]
fn nonzero_u8_generates_nonzero_values() {
    assert!(NonZeroU8::generator().check(|n| n.get() > 0).is_none());
}

#[test]
fn nonzero_i32_generates_nonzero_values() {
    assert!(NonZeroI32::generator().check(|n| n.get() != 0).is_none());
}

#[test]
fn nonzero_u8_full_range_has_correct_cardinality() {
    assert_eq!(NonZeroU8::generator().cardinality(), Some(255));
}

#[test]
fn nonzero_i8_full_range_has_correct_cardinality() {
    // -128..-1 = 128 values, 1..=127 = 127 values → 255 total
    assert_eq!(NonZeroI8::generator().cardinality(), Some(255));
}

// With #[check] macro:
#[check(_)]
fn nonzero_check_attribute_works(n: NonZeroU8) {
    assert!(n.get() > 0);
}
```
