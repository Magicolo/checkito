# `state.rs`: `char` Range Conversion Uses Saturating Arithmetic That Can Produce Incorrect Bounds

## Summary

The `From<ops::RangeFull> for Range<char>` and other range conversions for `char` in
`state.rs` use a closure-based `$up`/`$down` pair that calls `char::from_u32` with
`unwrap_or(char::REPLACEMENT_CHARACTER)`.  For boundary values near the surrogate range
(`U+D7FF` and `U+E000`), this causes the exclusive-range-to-inclusive conversion to skip
over surrogate code points correctly, but the logic for the `next_up` of `U+D7FF` (which
would be `U+D800`, a surrogate) maps to `REPLACEMENT_CHARACTER` (`U+FFFD`) rather than the
next valid character (`U+E000`), potentially excluding valid characters from generation.

## Affected Code

`checkito/src/state.rs` – the `char` ranges macro (approximately line 700):

```rust
ranges!(
    char,
    |value: char| char::from_u32(u32::saturating_add(value as _, 1))
        .unwrap_or(char::REPLACEMENT_CHARACTER),
    |value: char| char::from_u32(u32::saturating_sub(value as _, 1))
        .unwrap_or(char::REPLACEMENT_CHARACTER)
);
```

For exclusive ranges like `'a'..'z'`, the `$up` closure is applied to the exclusive end
bound to convert it to the inclusive equivalent.  For example:

```
'\u{D7FF}'..'z' (exclusive end 'z')
```

is straightforward, but consider:

```
'\u{D7FF}'..'z' (exclusive start '\u{D7FF}') 
```

After applying `$up` to the exclusive start `'\u{D7FF}'`:
```
u32::saturating_add(0xD7FF, 1) = 0xD800  (surrogate!)
char::from_u32(0xD800) = None
→ REPLACEMENT_CHARACTER (U+FFFD)
```

This means the range `'\u{D7FF}'..'z'` would be interpreted as starting at `U+FFFD` (≈ 65533)
rather than the correct `U+E000` (57344), the first character after the surrogate range.

## Why This Matters

A user who writes:

```rust
// All valid chars starting just after U+D7FF (exclusive)
let gen = '\u{D7FF}'..'\u{FFFE}';
```

expects the generator to produce `U+E000..=U+FFFD`.  Instead, due to the surrogate-skipping
behavior of the `unwrap_or`, the start would be mapped to `U+FFFD` (REPLACEMENT CHARACTER)
rather than `U+E000`.

Similarly, ranges that end just before `U+E000` (e.g., ending exclusively at `U+E000`) would
use the `$down` closure:

```
u32::saturating_sub(0xE000, 1) = 0xDFFF  (surrogate!)
char::from_u32(0xDFFF) = None
→ REPLACEMENT_CHARACTER (U+FFFD)
```

This is also wrong; the correct inclusive end should be `U+D7FF`.

## Root Cause

The `$up` and `$down` closures are designed to convert exclusive range bounds to inclusive
bounds.  For `char`, this means incrementing/decrementing the code point value.  However,
the surrogate range (`U+D800..=U+DFFF`) contains invalid `char` values.  Incrementing
`U+D7FF` (the last non-surrogate before the gap) correctly hits the first surrogate, which
maps to `None`.  But `unwrap_or(REPLACEMENT_CHARACTER)` maps to `U+FFFD` instead of
jumping over the surrogate gap to `U+E000`.

## Proposed Fix

The `$up` closure should skip over the surrogate range:

```rust
ranges!(
    char,
    |value: char| {
        let next = u32::saturating_add(value as u32, 1);
        // Skip over the surrogate range U+D800..=U+DFFF.
        let next = if next >= 0xD800 && next <= 0xDFFF { 0xE000 } else { next };
        char::from_u32(next).unwrap_or(char::MAX)
    },
    |value: char| {
        let prev = u32::saturating_sub(value as u32, 1);
        // Skip over the surrogate range going backwards.
        let prev = if prev >= 0xD800 && prev <= 0xDFFF { 0xD7FF } else { prev };
        char::from_u32(prev).unwrap_or(char::MIN)
    }
);
```

This ensures that:
- `next_char('\u{D7FF}')` → `'\u{E000}'` (skips surrogates)
- `prev_char('\u{E000}')` → `'\u{D7FF}'` (skips surrogates)

## Investigation Required

1. Verify the extent of the issue by writing a test that generates values from a range
   straddling the surrogate gap.
2. Check whether the `char` generation function (`State::char`) already handles the surrogate
   case (it does, mapping surrogate code points to `REPLACEMENT_CHARACTER`), and determine
   whether this fix would cause double-handling.

## Test Cases to Add

```rust
#[test]
fn char_range_exclusive_just_past_surrogate_gap_is_correct() {
    // '\u{D7FF}'..'z' with exclusive lower bound '\u{D7FF}'
    // After applying $up to U+D7FF, start should be U+E000 (not U+FFFD).
    let values: std::collections::HashSet<char> =
        ('\u{D7FF}'..'\u{E010}').samples(1000).collect();
    // U+D800..=U+DFFF should not appear (surrogates).
    for c in values {
        assert!(c < '\u{D800}' || c >= '\u{E000}', "surrogate U+{:04X} generated", c as u32);
    }
}

#[test]
fn char_exclusive_range_end_at_e000_does_not_produce_surrogates() {
    // Range 'a'..'\u{E000}' should produce up to U+D7FF, not U+DFFF.
    let values: std::collections::HashSet<char> =
        ('a'..'\u{E000}').samples(1000).collect();
    for c in values {
        assert!(c < '\u{D800}', "produced surrogate or post-surrogate char: U+{:04X}", c as u32);
    }
}
```
