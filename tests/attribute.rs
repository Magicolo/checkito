pub mod common;

use common::*;
use core::fmt;
use std::str::FromStr;

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

#[check(same("boba"))]
fn compiles_with_constant_str(_: &str) {}

#[check]
fn compiles_and_runs_once() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    assert_eq!(COUNT.fetch_add(1, Ordering::Relaxed), 0);
}

#[check(1, 'a')] // More than 1 attribute?
fn compiles_with_constants_and_runs_once(_: u8, _: char) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    assert_eq!(COUNT.fetch_add(1, Ordering::Relaxed), 0);
}

#[check(debug = true)]
fn compiles_with_debug_true() {}

#[check(debug = false)]
fn compiles_with_debug_false() {}

#[check(seed = 1234567890 / 100)]
fn compiles_with_seed() {}

#[check(reject = 1 + 123_098)]
fn compiles_with_reject() {}

#[check(accept = !0)]
fn compiles_with_accept() {}

#[check(count = 1)]
fn compiles_with_count() {}

#[check(true)]
const fn compiles_with_const(value: bool) -> bool {
    value
}

#[check(1usize)]
#[check(2u8)]
#[check('a')]
#[check("b")]
#[check('c'..'d')]
#[check(['a', 'b'].any().map(Option::unwrap))]
fn compiles_with_multiple_impl_generics(_a: impl FromStr) {}

#[check(1isize)]
#[check("a message")]
fn compiles_with_multiple_param_generics<T: fmt::Debug>(_a: T) {}

#[check(0)]
#[check(1)]
#[check(2)]
fn compiles_with_multiple_constants(value: usize) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    assert_eq!(COUNT.fetch_add(1, Ordering::Relaxed), value);
}
