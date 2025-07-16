#![cfg(feature = "regex")]

pub mod common;
use common::*;
use std::collections::HashSet;

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
    let set = dbg!(regex!("[a-z]{0,1}"))
        .checks(Ok::<_, ()>)
        .map(|result| result.item())
        .collect::<HashSet<_>>();
    set.contains("");
    for letter in 'a'..='z' {
        assert!(set.contains(letter.to_string().as_str()));
    }
}
