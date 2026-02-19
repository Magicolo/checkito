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

#[test]
fn unbounded_quantifiers_have_reasonable_limits() {
    // Test a* (0 or more)
    let generator = regex("a*", None).unwrap();
    for value in generator.samples(50) {
        assert!(value.len() <= 64); // Default REPEATS limit
    }

    // Test a+ (1 or more)
    let generator = regex("a+", None).unwrap();
    for value in generator.samples(50) {
        assert!(value.len() >= 1 && value.len() <= 64);
    }

    // Test a{1000,} (at least 1000 repetitions)
    let generator = regex("a{1000,}", None).unwrap();
    let value = generator.sample(0.5);
    assert_eq!(value.len(), 1000); // Should be exactly 1000 since high = max(64, 1000) = 1000
}

#[test]
fn nested_quantifiers_dont_become_zero() {
    // Deeply nested quantifiers like ((a*)*)* should still generate strings
    let generator = regex("((a*)*)*", None).unwrap();
    for value in generator.samples(20) {
        // Should generate valid strings (may be empty, but should not panic)
        assert!(value.chars().all(|c| c == 'a'));
    }

    // Test with multiple levels of nesting
    let generator = regex("(((a+)+)+)", None).unwrap();
    for value in generator.samples(20) {
        // Should generate non-empty strings with 'a' characters
        assert!(!value.is_empty());
        assert!(value.chars().all(|c| c == 'a'));
    }
}
