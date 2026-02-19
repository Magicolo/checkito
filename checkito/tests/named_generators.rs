pub mod common;
use common::*;
use checkito::standard::{character, number, with};

#[test]
fn letter_generator_works() {
    let gen = character::Letter::new();
    assert!(gen.check(|c| c.is_ascii_alphabetic()).is_none());
}

#[test]
fn digit_generator_works() {
    let gen = character::Digit::new();
    assert!(gen.check(|c| c.is_ascii_digit()).is_none());
}

#[test]
fn ascii_generator_works() {
    let gen = character::Ascii::new();
    assert!(gen.check(|c| c.is_ascii()).is_none());
}

#[test]
fn number_generator_works() {
    let gen = number::Number::<i32>::new();
    assert!(gen.check(|_| true).is_none());
}

#[test]
fn positive_generator_works() {
    let gen = number::Positive::<i32>::new();
    assert!(gen.check(|n| n >= 0).is_none());
}

#[test]
fn negative_generator_works() {
    let gen = number::Negative::<i32>::new();
    assert!(gen.check(|n| n <= 0).is_none());
}

#[test]
fn with_generator_works() {
    let gen = with::With::new(|| 42);
    assert!(gen.check(|n| n == 42).is_none());
}

#[test]
fn with_generator_struct() {
    #[derive(Debug, Clone, PartialEq)]
    struct MyStruct(i32);
    
    let gen = with::With::new(|| MyStruct(100));
    assert!(gen.check(|s| s == MyStruct(100)).is_none());
}

