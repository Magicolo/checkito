use checkito::{regex::Regex, *};

const COUNT: usize = 1024;

#[test]
fn range_shrinks() {
    let error = "[a-z]+"
        .parse::<Regex>()
        .unwrap()
        .check(COUNT, |item| !item.contains('w'))
        .err()
        .unwrap();
    assert!(error.original().len() > 5);
    assert!(error
        .original()
        .chars()
        .all(|symbol| symbol >= 'a' && symbol <= 'z'));
    assert_eq!(error.shrunk(), "w");
}
