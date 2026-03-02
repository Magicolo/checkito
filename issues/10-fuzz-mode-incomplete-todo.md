# TODO: Fuzzing Mode Is Incomplete / Under Consideration

## Summary

There is a `TODO` comment in `checkito/src/state.rs` indicating that a fuzzing mode (driven
by structured byte input rather than a PRNG) was considered but not implemented.  Fuzzing
mode would allow `checkito` generators to be driven by byte streams from coverage-guided
fuzzers (e.g., `cargo-fuzz` / `libFuzzer`), enabling a powerful hybrid property-testing /
fuzzing workflow.

## Affected Code

`checkito/src/state.rs` – approximately line 113:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
enum Mode {
    // TODO: Can I use this for fuzzing? Add a `Fuzz(Box<dyn Iterator<Item = byte>>)`?
    Random(Rng),
    Exhaustive(u128),
}
```

## Background

Coverage-guided fuzzers (AFL, libFuzzer, Honggfuzz) mutate a byte corpus to maximize code
coverage.  Many property-testing libraries add a "fuzz mode" that interprets the fuzzer's
byte stream as the source of entropy for their generators.  This means:

1. The fuzzer finds inputs that cover new code paths.
2. The property-testing library converts those bytes into structured values.
3. Failing inputs from the fuzzer can be replayed as property test seeds.

Popular Rust implementations include `bolero`'s `TypeGenerator`, `arbitrary::Arbitrary`, and
`proptest`'s `fuzz` integration.

## Design Sketch for Fuzzing Mode

### Core Idea

Add a `Fuzz` variant to `Mode` that consumes bytes from a provided buffer:

```rust
enum Mode {
    Random(Rng),
    Exhaustive(u128),
    Fuzz { bytes: Box<[u8]>, cursor: usize },
}
```

Each generator call reads a number of bytes appropriate to its output range.  For example:

```rust
fn u8_fuzz(bytes: &[u8], cursor: &mut usize) -> u8 {
    let byte = bytes.get(*cursor).copied().unwrap_or(0);
    *cursor += 1;
    byte
}
```

For larger integers:

```rust
fn u64_fuzz(bytes: &[u8], cursor: &mut usize) -> u64 {
    let end = (*cursor + 8).min(bytes.len());
    let mut buf = [0u8; 8];
    buf[..end - *cursor].copy_from_slice(&bytes[*cursor..end]);
    *cursor = end;
    u64::from_le_bytes(buf)
}
```

For range-bounded integers, apply `wrapping_rem` to map to the range (or use a faster
power-of-2 approach for ranges that are powers of two):

```rust
fn u8_range_fuzz(bytes: &[u8], cursor: &mut usize, range: Range<u8>) -> u8 {
    let raw = u8_fuzz(bytes, cursor);
    let span = range.end().wrapping_sub(range.start()).wrapping_add(1);
    range.start().wrapping_add(raw.wrapping_rem(span))
}
```

### Entry Point

```rust
impl State {
    /// Create a `State` that consumes bytes from a fuzz corpus input.
    pub fn fuzz(bytes: &[u8]) -> Self {
        Self {
            mode: Mode::Fuzz {
                bytes: bytes.to_vec().into_boxed_slice(),
                cursor: 0,
            },
            sizes: Sizes::DEFAULT,
            index: 0,
            count: 1,
            limit: 0,
            depth: 0,
            seed: 0,
        }
    }
}
```

### `libFuzzer` / `cargo-fuzz` Integration

A user would write a fuzz target that converts the fuzzer's byte buffer to structured values:

```rust
// fuzz/fuzz_targets/my_property.rs
#![no_main]

use checkito::{Generate, state::State};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut state = State::fuzz(data);
    let value: i32 = (i32::MIN..=i32::MAX).generate(&mut state).item();
    // Test the property:
    assert!(my_function(value).is_ok());
});
```

## Considerations

### Shrinking in Fuzz Mode

Unlike random mode, fuzz mode gets its entropy from the fuzzer.  Shrinking in fuzz mode
would need to reduce the byte buffer (e.g., truncating, zeroing bytes) rather than using the
existing binary-search shrinker.  Alternatively, fuzz mode could be "generation-only" with
shrinking delegated to the fuzzer's built-in minimization.

### `Arbitrary` Trait Interaction

The `arbitrary` crate already defines a standard way to derive structured value generation
from byte slices.  If `checkito` adds fuzz mode, it could optionally expose an `Arbitrary`
implementation that uses `State::fuzz`, enabling integration with `cargo-fuzz` without
duplicating logic.

### Feature Flag

Fuzz mode should be behind a `fuzz` feature flag since it requires `libfuzzer_sys` or similar
in the dependency tree when enabled.

## Investigation Required

Before implementing, the following questions need answers:

1. How should byte exhaustion be handled?  Options: wrap around, pad with zeros, return
   `None`.
2. Should shrinking be supported in fuzz mode?  If so, what strategy?
3. How should floats be handled?  (Current random mode maps total-order bit patterns.)
4. Is an `Arbitrary` adapter desirable?

## Fix Plan

1. Design the `Mode::Fuzz` variant and byte-consumption helpers.
2. Implement generation methods (`u8`, `i32`, `f64`, etc.) in fuzz mode.
3. Add a `State::fuzz(bytes: &[u8])` constructor.
4. Optionally expose an `Arbitrary` impl behind `cfg(feature = "arbitrary")`.
5. Add fuzz targets in `fuzz/` directory using `cargo-fuzz`.
6. Document the fuzz workflow in `README.md`.
