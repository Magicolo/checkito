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

#[check(_, _, _, _)]
fn compiles_with_multiple_discarded_arguments(
    _first: u8,
    _second: String,
    _third: Vec<()>,
    _fourth: [isize; 12],
) {
}

#[check(..)]
fn compiles_with_multiple_rest_arguments(
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
#[should_panic]
fn fails_on_specific_input(left: String, right: String) {
    if left.len() + right.len() > 10 {
        assert_eq!(left.contains('z'), right.contains('Z'));
    }
}

#[check(1, 'a')]
fn compiles_with_constants_and_runs_once(_: u8, _: char) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    assert_eq!(COUNT.fetch_add(1, Ordering::Relaxed), 0);
}
