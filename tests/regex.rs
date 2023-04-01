use checkito::{regex::Regex, Generate};
use std::error;

const COUNT: usize = 1024;

#[test]
fn generate_matches_regex() -> Result<(), Box<dyn error::Error>> {
    const REGEX: &'static str = "((a|b)*[A-Z]*[\\u0000-\\u0FFF^\\u00AF-\\u00FF]*c{4}d{2,10})+";
    let matcher = regex::RegexBuilder::new(REGEX).build().unwrap();
    REGEX
        .parse::<Regex>()
        .unwrap()
        .check(COUNT, |item| matcher.is_match(item))?;
    Ok(())
}

#[test]
fn range_shrinks() {
    let error = "[a-z]+"
        .parse::<Regex>()
        .unwrap()
        .check(COUNT, |item| {
            !dbg!(item).contains('w') || !item.contains('y')
        })
        .err()
        .unwrap();
    assert!(error.original().len() > 5);
    assert!(error
        .original()
        .chars()
        .all(|symbol| symbol >= 'a' && symbol <= 'z'));
    assert!(error.shrunk() == "wy" || error.shrunk() == "yw");
}
