pub mod common;
use common::*;

#[check('a'..='z')]
fn compiles_range_expression(value: char) {
    assert!(value.is_ascii_lowercase());
}

#[check(letter())]
fn compiles_with_builtin_generator(value: char) {
    assert!(value.is_ascii_alphabetic())
}

#[check]
fn compiles_with_output() -> bool {
    true
}

#[check]
#[should_panic]
fn compiles_with_should_panic() -> bool {
    false
}

#[check('A'..='Z')]
fn compiles_with_input_output(value: char) -> bool {
    value.is_ascii_uppercase()
}

#[check]
fn compiles_with_no_argument() {}

#[check]
fn compiles_with_multiple_arguments(
    _first: u8,
    _second: String,
    _third: Vec<()>,
    _fourth: [isize; 12],
) {
}

#[check("[0-9]{5}")]
fn compiles_with_regex_input(value: String) {
    assert!(value.len() >= 5);
    assert!(value.chars().all(|value| value.is_numeric()));
}

#[check(Generate::collect('a'..='z'), Generate::collect('A'..='Z'))]
fn fails_on_specific_input(left: String, right: String) {
    if left.len() + right.len() > 10 {
        assert_eq!(left.contains('z'), right.contains('Z'));
    }
}
