pub mod common;
use common::*;
use core::fmt;
use std::{
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
};

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
    _first: (u8,),
    _second: String,
    _third: Vec<char>,
    _fourth: [isize; 12],
) {
}

#[check(..)]
fn compiles_with_multiple_rest_arguments(
    _first: (u8,),
    _second: String,
    _third: Vec<char>,
    _fourth: [isize; 12],
) {
}

#[check(_, ..)]
fn compiles_with_discard_and_rest_arguments(
    _first: (u8,),
    _second: String,
    _third: Vec<char>,
    _fourth: [isize; 12],
) {
}

#[check(regex!("[0-9]{5}"))]
fn compiles_with_regex_input(value: String) {
    assert!(value.len() >= 5);
    assert!(value.chars().all(|value| value.is_numeric()));
}

#[check("a string")]
fn compiles_with_constant_str(_: &str) {}

#[check]
fn compiles_and_runs_once() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    assert_eq!(COUNT.fetch_add(1, Ordering::Relaxed), 0);
}

#[check(1u8, 'a')] // More than 1 attribute?
fn compiles_with_constants_and_runs_once(_: u8, _: char) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    assert_eq!(COUNT.fetch_add(1, Ordering::Relaxed), 0);
}

#[check(debug = true)]
fn compiles_with_debug_true() {}

#[check(debug = false)]
fn compiles_with_debug_false() {}

#[check(color = true)]
fn compiles_with_color_true() {}

#[check(color = false)]
fn compiles_with_color_false() {}

#[check(verbose = true)]
fn compiles_with_verbose_true() {}

#[check(verbose = false)]
fn compiles_with_verbose_false() {}

#[check(generate.seed = 1234567890 / 100)]
fn compiles_with_generate_seed() {}

#[check(generate.size = 0.25)]
fn compiles_with_generate_size_constant() {}

#[check(generate.size = 0.25..0.75)]
fn compiles_with_generate_size_range() {}

#[check(generate.items = false)]
fn compiles_with_generate_items() {}

#[check(generate.count = 100)]
fn compiles_with_generate_count() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    assert!(COUNT.fetch_add(1, Ordering::Relaxed) < 100);
}

#[check(shrink.count = 1 + 123_098)]
fn compiles_with_shrink_count() {}

#[check(shrink.items = false)]
fn compiles_with_shrink_items() {}

#[check(shrink.errors = true)]
fn compiles_with_shrink_errors() {}

#[check(true)]
const fn compiles_with_const(value: bool) -> bool {
    value
}

#[check(1usize)]
#[check(2u8)]
#[check('a')]
#[check("b".to_string())]
#[check('c'..'d')]
#[check(['a', 'b'].any().map(Option::unwrap))]
fn compiles_with_multiple_impl_generics(_a: impl FromStr) {}

#[check(1isize)]
#[check("a message")]
fn compiles_with_multiple_param_generics<T: fmt::Debug>(_a: T) {}

#[check(0usize)]
#[check(1usize)]
#[check(2usize)]
fn compiles_with_multiple_constants(value: usize) {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    assert_eq!(COUNT.fetch_add(1, Ordering::Relaxed), value);
}

struct A;
#[derive(Clone)]
struct B;
#[check(with(|| A), Same(B), debug = false)]
fn compiles_with_non_debug_parameter(_a: A, _b: B) {}

#[check(Option::<usize>::generator().map(Option::unwrap))]
#[should_panic]
fn panics_with_option_unwrap(_: usize) {}