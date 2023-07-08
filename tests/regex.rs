pub mod common;
use checkito::regex::Regex;
use common::*;

#[test]
fn generate_matches_regex() -> Result {
    const PATTERN: &str = "((a|b)*[A-Z]*[\\u0000-\\u0FFF^\\u00AF-\\u00FF]*c{4}d{2,10})+";
    let matcher = ::regex::RegexBuilder::new(PATTERN).build().unwrap();
    PATTERN
        .parse::<Regex>()
        .unwrap()
        .check(COUNT, |item| matcher.is_match(item))?;
    Ok(())
}

#[test]
fn generate_constant() -> Result {
    "[a-zA-Z0-9_]+"
        .parse::<Regex>()
        .unwrap()
        .flat_map(|pattern| (pattern.parse::<Regex>().unwrap(), pattern))
        .check(COUNT, |(item, pattern)| item == pattern)?;
    Ok(())
}

#[test]
fn range_shrinks() {
    let error = "[a-z]+"
        .parse::<Regex>()
        .unwrap()
        .check(COUNT, |item| !item.contains('w') || !item.contains('y'))
        .err()
        .unwrap();
    assert!(error.original.len() > 5);
    assert!(error
        .original
        .chars()
        .all(|symbol| symbol.is_ascii_lowercase()));
    assert!(error.shrunk() == "wy" || error.shrunk() == "yw");
}
