#![cfg(feature = "regex")]

pub mod common;
use common::*;

#[test]
fn generate_matches_regex() {
    const PATTERN: &str = "((a|b)*[A-Z]*[\\u0000-\\u0FFF^\\u00AF-\\u00FF]*c{4}d{2,10})+";
    let matcher = ::regex::RegexBuilder::new(PATTERN).build().unwrap();
    assert!(
        regex(PATTERN, None)
            .unwrap()
            .check(|item| matcher.is_match(&item))
            .is_none()
    );
}

#[test]
fn generate_constant() {
    assert!(
        regex!("[a-zA-Z0-9_]+")
            .flat_map(|pattern| (regex(&pattern, None).unwrap(), pattern))
            .check(|(item, pattern)| item == pattern)
            .is_none()
    );
}

#[test]
fn range_shrinks() {
    let fail = regex!("[a-z]+")
        .check(|item| !item.contains('w') || !item.contains('y'))
        .unwrap();
    assert!(fail.item.chars().all(|symbol| symbol.is_ascii_lowercase()));
    assert!(fail.item == "wy" || fail.item == "yw");
}

#[test]
fn generates_exhaustively() {
    let values = regex!("[a-c]{0,2}")
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();

    assert_eq!(values.len(), 13);
    assert!(values.contains(&"".to_owned()));
    for first in ['a', 'b', 'c'] {
        assert!(values.contains(&first.to_string()));
        for second in ['a', 'b', 'c'] {
            assert!(values.contains(&format!("{first}{second}")));
        }
    }
}

// Test for Fix #1: Byte ranges should not panic and produce valid UTF-8
#[test]
fn regex_handles_byte_ranges_safely() {
    // Should not panic or produce invalid chars
    // All u8 values (0-255) map to valid Unicode scalar values U+0000 to U+00FF
    let gen = regex(r"[\x00-\xFF]", None).unwrap();
    for s in gen.samples(100) {
        // All generated strings should be valid UTF-8
        assert!(s.is_char_boundary(0));
        assert_eq!(s.chars().count(), 1);
        // Should contain only valid characters in the range U+0000 to U+00FF
        let ch = s.chars().next().unwrap();
        assert!((ch as u32) <= 0xFF, "Character U+{:04X} is out of expected range", ch as u32);
    }
}

// Test for Fix #2: Unbounded quantifiers should have reasonable limits
#[test]
fn unbounded_quantifiers_have_reasonable_limits() {
    // Test a* (0 or more)
    let gen = regex("a*", None).unwrap();
    for s in gen.samples(50) {
        assert!(s.len() <= 64); // Default REPEATS limit
    }

    // Test a+ (1 or more)
    let gen = regex("a+", None).unwrap();
    for s in gen.samples(50) {
        assert!(s.len() >= 1 && s.len() <= 64);
    }

    // Test a{1000,} (at least 1000 repetitions)
    let gen = regex("a{1000,}", None).unwrap();
    let s = gen.sample(0.5);
    assert_eq!(s.len(), 1000); // Should be exactly 1000 since high = max(64, 1000) = 1000
}

// Test for Fix #3: Deeply nested quantifiers should not become zero
#[test]
fn nested_quantifiers_dont_become_zero() {
    // Deeply nested quantifiers like ((a*)*)* should still generate strings
    let gen = regex("((a*)*)*", None).unwrap();
    for s in gen.samples(20) {
        // Should generate valid strings (may be empty, but should not panic)
        assert!(s.chars().all(|c| c == 'a'));
    }

    // Test with multiple levels of nesting
    let gen = regex("(((a+)+)+)", None).unwrap();
    for s in gen.samples(20) {
        // Should generate non-empty strings with 'a' characters
        assert!(!s.is_empty());
        assert!(s.chars().all(|c| c == 'a'));
    }
}

// Test that ASCII byte ranges work correctly
#[test]
fn ascii_byte_ranges_work() {
    let gen = regex(r"[\x20-\x7E]", None).unwrap(); // Printable ASCII
    for s in gen.samples(100) {
        assert_eq!(s.chars().count(), 1);
        let ch = s.chars().next().unwrap();
        assert!(ch >= ' ' && ch <= '~');
    }
}

