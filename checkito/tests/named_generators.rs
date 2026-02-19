pub mod common;
use checkito::primitive::Constant;
use checkito::standard::{character, number, with};
use common::*;

// Compile-time assertion: Named generators can be used in struct fields
struct GeneratorConfig {
    letter_gen: character::Letter,
    digit_gen: character::Digit,
    ascii_gen: character::Ascii,
    num_gen: number::Number<i32>,
    positive_gen: number::Positive<i32>,
    negative_gen: number::Negative<i64>,
    with_gen: with::With<fn() -> i32>,
}

// Compile-time assertion: Named generators can be used in type aliases
type LetterAlias = character::Letter;
type NumberAlias = number::Number<i64>;
type PositiveAlias = number::Positive<i32>;

#[test]
fn letter_generator_works() {
    let gen = character::Letter::VALUE;
    assert!(gen.check(|c| c.is_ascii_alphabetic()).is_none());
}

#[test]
fn digit_generator_works() {
    let gen = character::Digit::VALUE;
    assert!(gen.check(|c| c.is_ascii_digit()).is_none());
}

#[test]
fn ascii_generator_works() {
    let gen = character::Ascii::VALUE;
    assert!(gen.check(|c| c.is_ascii()).is_none());
}

#[test]
fn number_generator_works() {
    let gen = number::Number::<i32>::VALUE;
    assert!(gen.check(|_| true).is_none());
}

#[test]
fn positive_generator_works() {
    let gen = number::Positive::<i32>::VALUE;
    assert!(gen.check(|n| n >= 0).is_none());
}

#[test]
fn negative_generator_works() {
    let gen = number::Negative::<i32>::VALUE;
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

#[test]
fn struct_fields_can_hold_generators() {
    // Verify that generators can be stored in struct fields
    let config = GeneratorConfig {
        letter_gen: character::Letter::VALUE,
        digit_gen: character::Digit::VALUE,
        ascii_gen: character::Ascii::VALUE,
        num_gen: number::Number::VALUE,
        positive_gen: number::Positive::VALUE,
        negative_gen: number::Negative::VALUE,
        with_gen: with::With::new(|| 42),
    };

    // Verify they work when stored in a struct
    assert!(config.letter_gen.check(|c| c.is_ascii_alphabetic()).is_none());
    assert!(config.positive_gen.check(|n| n >= 0).is_none());
}

#[test]
fn type_aliases_work() {
    // Verify that type aliases can be used
    let letter: LetterAlias = character::Letter::VALUE;
    let number: NumberAlias = number::Number::VALUE;
    let positive: PositiveAlias = number::Positive::VALUE;

    assert!(letter.check(|c| c.is_ascii_alphabetic()).is_none());
    assert!(number.check(|_| true).is_none());
    assert!(positive.check(|n| n >= 0).is_none());
}

