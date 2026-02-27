# Missing `FullGenerate` for `Duration`, `Instant`, and Other Common Standard Library Types

## Summary

The library provides `FullGenerate` for all primitive types, collections, and smart
pointers, but several commonly used standard library types that are natural candidates for
property testing do not have default generators.  The most frequently needed are:

- `std::time::Duration`
- `std::net::Ipv4Addr`, `std::net::Ipv6Addr`, `std::net::IpAddr`
- `std::net::SocketAddrV4`, `std::net::SocketAddrV6`, `std::net::SocketAddr`
- `core::ops::Range<T>`, `core::ops::RangeInclusive<T>` (as generated values, not generators)
- `core::num::Saturating<T>`, `core::num::Wrapping<T>`

This issue focuses primarily on `Duration`, `Saturating<T>`, and `Wrapping<T>` as the most
commonly needed in property tests of numeric or time-related code.

## Motivation

### `Duration`

`std::time::Duration` appears in almost any code dealing with timeouts, retries, or
scheduling.  Property testing such code requires generating arbitrary durations.  Without
`FullGenerate for Duration`, users must manually construct a generator:

```rust
// Current workaround:
let gen = (u64::generator(), u32::generator())
    .map(|(secs, nanos)| Duration::new(secs, nanos % 1_000_000_000));
```

This is verbose and easy to get wrong (the nanosecond component must be `< 1_000_000_000`).

A built-in implementation would be:

```rust
impl FullGenerate for Duration {
    type Item = Duration;
    type Generator = /* … */;

    fn generator() -> Self::Generator {
        (u64::generator(), (0u32..1_000_000_000u32))
            .map(|(secs, nanos)| Duration::new(secs, nanos))
    }
}
```

### `Wrapping<T>` and `Saturating<T>`

These wrapper types are thin wrappers around integer primitives with different arithmetic
semantics.  Since `T: FullGenerate` for all integer types, the implementation is trivial:

```rust
impl<T: FullGenerate<Item = T>> FullGenerate for Wrapping<T> {
    type Item = Wrapping<T>;
    type Generator = Map<T::Generator, fn(T) -> Wrapping<T>>;

    fn generator() -> Self::Generator {
        T::generator().map(Wrapping)
    }
}
```

### Network Types

`Ipv4Addr` can be generated from four independent `u8` values:

```rust
impl FullGenerate for Ipv4Addr {
    type Generator = Map<[u8; 4], fn([u8; 4]) -> Ipv4Addr>;

    fn generator() -> Self::Generator {
        [u8::generator(); 4].map(|[a, b, c, d]| Ipv4Addr::new(a, b, c, d))
    }
}
```

## Investigation Required

Before implementing, determine:

1. Which types are most commonly requested by users (priority: `Duration`, then `Wrapping`,
   `Saturating`, then network types).
2. Whether to add these to the main `standard.rs` or a separate `stdlib.rs` module.
3. Whether to gate network types behind a feature flag (they require `std`, which is already
   a requirement for most of the library's features).

## Fix Plan

1. Add `FullGenerate for Duration` in `standard.rs`.
2. Add `FullGenerate for Wrapping<T>` and `Saturating<T>` in `standard.rs`.
3. Add `FullGenerate for Ipv4Addr`, `Ipv6Addr`, `IpAddr`, `SocketAddr*` behind a
   `net` feature flag or unconditionally (since `std::net` is always available in `std`).
4. Expose the generators in the prelude if appropriate.
5. Add tests for each new generator.

## Test Cases to Add

```rust
#[test]
fn duration_generates_valid_values() {
    use std::time::Duration;
    assert!(Duration::generator().check(|d| d.subsec_nanos() < 1_000_000_000).is_none());
}

#[test]
fn wrapping_u32_generates_full_range() {
    use core::num::Wrapping;
    assert!(Wrapping::<u32>::generator().check(|_| true).is_none());
    let cardinality = Wrapping::<u32>::generator().cardinality();
    assert_eq!(cardinality, u32::generator().cardinality());
}

#[test]
fn ipv4_generates_valid_addresses() {
    use std::net::Ipv4Addr;
    assert!(Ipv4Addr::generator().check(|_| true).is_none());
}
```
