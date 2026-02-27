# Regex Byte-Class Handling May Generate Characters Outside the Intended Range

## Summary

When a regex contains a byte class (e.g., `[a-z\x80-\xff]`), the `Regex` generator in
`checkito/src/regex.rs` converts `ClassBytesRange` entries to `char` ranges using
`char::from_u32`.  For bytes in the range `0x80–0xFF`, the resulting `char` values are in
the Latin-1 Supplement Unicode block, not raw bytes.  This is semantically different from
what the regex byte class specifies, and it can generate strings that do not actually match
the original regular expression when validated with the `regex` crate.

## Affected Code

`checkito/src/regex.rs` – the `From<&ClassBytesRange> for Regex` implementation:

```rust
impl From<&ClassBytesRange> for Regex {
    fn from(value: &ClassBytesRange) -> Self {
        let start = char::from_u32(value.start() as u32).unwrap_or(char::REPLACEMENT_CHARACTER);
        let end = char::from_u32(value.end() as u32).unwrap_or(char::REPLACEMENT_CHARACTER);
        Regex::Range(Range(start, end))
    }
}
```

`ClassBytesRange` stores byte values in the range `0x00–0xFF`.  `char::from_u32(n)` for
`n` in `0x00–0x7F` returns ASCII characters, and for `n` in `0x80–0xFF` returns the
corresponding Latin-1 Supplement codepoints (`U+0080..=U+00FF`).

## Why This Is a Problem

### Mismatch With Regex Semantics

A regex like `[^\x00-\x7F]` (any byte with the high bit set) is intended to match single
bytes `0x80..=0xFF`.  When `checkito` converts this to a `char` range, it generates
**two-byte UTF-8 sequences** (e.g., `U+0080` encodes as the two bytes `0xC2 0x80`), not the
single bytes `0x80..=0xFF`.

The generated `String` would therefore contain valid Unicode but the resulting UTF-8 encoding
does not match the original byte-level regex.

### Concrete Example

```rust
use checkito::regex;

// This regex should match strings where all chars are in the byte range 0x80..0xFF.
// But when checkito converts ClassBytesRange to char, it generates
// U+0080..=U+00FF (Latin-1 Supplement), which encodes as 2-byte UTF-8 sequences.
// The regex crate matches against UTF-8 bytes, so the generated strings may or
// may not match depending on whether the regex is compiled in Unicode or byte mode.
let gen = regex!(r"[\x80-\xFF]+");
```

### Impact on Test Validity

If a user writes:

```rust
#[check(regex!(r"[\x80-\xFF]+"))]
fn my_property(s: String) {
    // They expect s to contain only bytes in 0x80..=0xFF...
    // But the regex crate (in Unicode mode) actually compiles this differently.
}
```

The generated strings may not match what the user expected.

## Investigation Required

The issue is partly in `regex_syntax`'s `ClassBytesRange` and partly in how `checkito`
converts it.  The `regex_syntax` crate uses byte classes for patterns compiled with
`(?-u)` (non-Unicode mode), but in Unicode mode the same `\x80-\xFF` pattern is compiled to
a Unicode range.

**Action items:**
1. Confirm whether `checkito`'s `Regex::from_hir` path is ever triggered with `ClassBytesRange`
   for Unicode-mode patterns.
2. If `ClassBytesRange` only appears for non-Unicode patterns, decide whether to:
   a. Skip byte-class ranges (return `Regex::Empty`) since non-Unicode byte strings cannot be
      represented as Rust `String`.
   b. Map byte ranges to ASCII characters for `0x00–0x7F` and skip `0x80–0xFF` ranges.
3. Add a test that validates generated strings against the original regex pattern.

## Fix Plan

### Option A – Skip non-ASCII byte ranges

Map only `0x00–0x7F` byte ranges to ASCII chars and return `Regex::Empty` for the rest:

```rust
impl From<&ClassBytesRange> for Regex {
    fn from(value: &ClassBytesRange) -> Self {
        let start = value.start();
        let end = value.end();
        // Only map pure-ASCII byte ranges to char ranges.
        // Non-ASCII byte classes (0x80-0xFF) cannot be represented as valid
        // UTF-8 strings without changing semantics.
        if end <= 0x7F {
            let start_char = char::from_u32(start as u32)
                .unwrap_or(char::REPLACEMENT_CHARACTER);
            let end_char = char::from_u32(end as u32)
                .unwrap_or(char::REPLACEMENT_CHARACTER);
            Regex::Range(Range(start_char, end_char))
        } else if start <= 0x7F {
            // Partially overlapping with ASCII: only generate ASCII portion.
            let end_char = char::from_u32(0x7F).unwrap();
            let start_char = char::from_u32(start as u32)
                .unwrap_or(char::REPLACEMENT_CHARACTER);
            Regex::Range(Range(start_char, end_char))
        } else {
            // Entirely non-ASCII byte class: cannot represent in UTF-8 String.
            Regex::Empty
        }
    }
}
```

### Option B – Convert to equivalent Unicode ranges

The Latin-1 Supplement block (`U+0080..=U+00FF`) is the Unicode equivalent of the Latin-1
byte range.  Converting to these chars is semantically correct for patterns like `[\x80-\xFF]`
that intend "Latin-1 characters", just not for raw-byte regex.

Document this behavior explicitly:

```rust
impl From<&ClassBytesRange> for Regex {
    fn from(value: &ClassBytesRange) -> Self {
        // Note: byte class ranges are mapped to the corresponding Unicode codepoints.
        // For values 0x80..=0xFF, this produces Latin-1 Supplement characters
        // (U+0080..=U+00FF), which encode as two UTF-8 bytes.  This is the best
        // approximation for Rust String generation, which requires valid UTF-8.
        let start = char::from_u32(value.start() as u32).unwrap_or(char::REPLACEMENT_CHARACTER);
        let end = char::from_u32(value.end() as u32).unwrap_or(char::REPLACEMENT_CHARACTER);
        Regex::Range(Range(start, end))
    }
}
```

### Recommended

**Option A** is more conservative and avoids generating strings that won't match byte-oriented
regexes.  **Option B** preserves current behavior and just documents it.  The right choice
depends on the expected use cases for byte-class regexes in `checkito`.

## Test Cases to Add

```rust
#[test]
fn regex_byte_class_ascii_generates_valid_matches() {
    use regex::Regex as Re;
    let pattern = r"[\x41-\x5A]+"; // A-Z in byte notation
    let re = Re::new(pattern).unwrap();
    let gen = checkito::prelude::regex(pattern, None).unwrap();
    gen.check(|s: String| re.is_match(&s)).unwrap_or_default();
}

#[test]
fn regex_generated_strings_match_original_pattern() {
    // Test that all generated strings actually match the regex they were generated from.
    use regex::Regex as Re;
    let pattern = r"[a-zA-Z0-9]{3,10}";
    let re = Re::new(pattern).unwrap();
    let gen = checkito::prelude::regex(pattern, None).unwrap();
    assert!(gen.check(|s: String| re.is_match(&s)).is_none());
}
```
