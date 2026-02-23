# Issue: Fuzzing Mode Investigation

## Summary

The `Mode` enum in `checkito/src/state.rs` has a TODO comment suggesting that
it could support a fuzzing mode where values are derived from a byte stream
rather than from a random number generator. This would allow `checkito` to
integrate with coverage-guided fuzzers such as `libFuzzer` or `cargo-fuzz`.

## Location

- `checkito/src/state.rs:85`

```rust
enum Mode {
    // TODO: Can I use this for fuzzing? Add a `Fuzz(Box<dyn Iterator<Item = byte>>)`? Or
    // maybe fuzz through the `Random` object?
    Random(Rng),
    Exhaustive(u128),
}
```

## Motivation

Coverage-guided fuzzing (e.g. `cargo-fuzz` / `libFuzzer`) provides a corpus of
byte sequences and mutates them to maximise code coverage. Integrating with
this approach would let `checkito` generators interpret fuzzer-provided bytes
as structured inputs, combining the benefits of:
- Fuzzer-guided corpus exploration (coverage feedback).
- `checkito`'s structured generation and shrinking.

This is the approach taken by libraries such as `arbitrary` and
`bolero`/`bolero-generator`.

## Design Options

### Option A: `Fuzz(Box<dyn Iterator<Item = u8>>)`

Add a third `Mode` variant that reads bytes from an iterator:

```rust
enum Mode {
    Random(Rng),
    Exhaustive(u128),
    Fuzz(FuzzSource),  // wraps &[u8] or Box<dyn Iterator<Item = u8>>
}
```

Each `state.$integer()` call would consume the appropriate number of bytes
from the iterator to construct the value. This requires all generation
primitives to have a byte-consuming path.

**Pros**: Clean separation of concerns; existing generators work unchanged.
**Cons**: High implementation cost; byte stream alignment is fragile.

### Option B: Seed-from-bytes via `Random`

Interpret the fuzzer's byte slice as a seed for the existing `Rng`:

```rust
let seed = u64::from_le_bytes(bytes[0..8].try_into().unwrap_or([0;8]));
State::random(0, 1, Sizes::DEFAULT, seed)
```

This requires no changes to `Mode` but provides poor fuzzer guidance (a
one-bit change in the seed produces a completely different value sequence).

**Pros**: Zero implementation cost.
**Cons**: Coverage feedback is ineffective; the fuzzer cannot make meaningful
  mutations.

### Option C: Integrate with the `arbitrary` crate

Implement `arbitrary::Arbitrary` for the `Generate` types, or add a
`from_arbitrary` adapter. This allows using `cargo-fuzz` directly with
`checkito`'s type system.

**Pros**: Leverages existing ecosystem; users can choose their fuzzer.
**Cons**: Adds a dependency on `arbitrary`; changes the public API.

## Investigation Required

1. Survey existing fuzzing integration patterns in Rust property-testing
   libraries (`proptest`, `quickcheck`, `bolero`).
2. Prototype Option A or Option C to assess implementation cost.
3. Determine whether `no_std` compatibility is preserved.
4. Decide whether fuzzing should be a first-class mode or an optional feature.

## Related

- `checkito/src/state.rs:85` (TODO comment)
- `cargo-fuzz` / `libFuzzer` integration
- `arbitrary` crate: https://docs.rs/arbitrary
