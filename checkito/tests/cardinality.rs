pub mod common;
use checkito::Generate;
use common::*;

#[test]
fn u8_full_range_has_correct_cardinality() {
    let generator = u8::generator();
    assert_eq!(generator.cardinality(), Some(256));
}

#[test]
fn i8_full_range_has_correct_cardinality() {
    let generator = i8::generator();
    assert_eq!(generator.cardinality(), Some(256));
}

#[test]
fn u16_full_range_has_correct_cardinality() {
    let generator = u16::generator();
    assert_eq!(generator.cardinality(), Some(65536));
}

#[test]
fn i16_full_range_has_correct_cardinality() {
    let generator = i16::generator();
    assert_eq!(generator.cardinality(), Some(65536));
}

#[test]
fn u32_full_range_has_correct_cardinality() {
    let generator = u32::generator();
    assert_eq!(generator.cardinality(), Some(4294967296));
}

#[test]
fn i32_full_range_has_correct_cardinality() {
    let generator = i32::generator();
    assert_eq!(generator.cardinality(), Some(4294967296));
}

#[test]
fn u64_full_range_has_correct_cardinality() {
    let generator = u64::generator();
    assert_eq!(generator.cardinality(), Some(18446744073709551616));
}

#[test]
fn i64_full_range_has_correct_cardinality() {
    let generator = i64::generator();
    assert_eq!(generator.cardinality(), Some(18446744073709551616));
}

#[test]
fn u128_full_range_has_correct_cardinality() {
    let generator = u128::generator();
    // u128 has 2^128 values, which overflows u128; cardinality should be None
    assert_eq!(generator.cardinality(), None);
}

#[test]
fn i128_full_range_has_correct_cardinality() {
    let generator = i128::generator();
    // i128 has 2^128 values, which overflows u128; cardinality should be None
    assert_eq!(generator.cardinality(), None);
}

#[test]
fn char_full_range_has_correct_cardinality() {
    let generator = char::generator();
    // char has valid Unicode code points from 0 to char::MAX
    let expected = u128::wrapping_sub(char::MAX as u128, 0).wrapping_add(1);
    assert_eq!(generator.cardinality(), Some(expected));
}

#[test]
fn custom_u8_range_has_correct_cardinality() {
    let generator = 10u8..=20;
    // Range from 10 to 20 inclusive = 11 values
    assert_eq!(generator.cardinality(), Some(11));
}

#[test]
fn custom_i8_range_has_correct_cardinality() {
    let generator = -5i8..=5;
    // Range from -5 to 5 inclusive = 11 values
    assert_eq!(generator.cardinality(), Some(11));
}

#[test]
fn custom_char_range_has_correct_cardinality() {
    let generator = 'a'..='z';
    // Range from 'a' to 'z' inclusive = 26 values
    assert_eq!(generator.cardinality(), Some(26));
}

#[test]
fn single_value_range_has_cardinality_one() {
    let generator = 42u8..=42;
    assert_eq!(generator.cardinality(), Some(1));
}

#[test]
fn full_usize_has_correct_cardinality() {
    let generator = usize::generator();
    let range = (usize::MAX as u128) - (usize::MIN as u128);
    let expected = range.checked_add(1);
    match expected {
        Some(value) => assert_eq!(generator.cardinality(), Some(value)),
        None => assert_eq!(generator.cardinality(), None),
    }
}

#[test]
fn full_isize_has_correct_cardinality() {
    let generator = isize::generator();
    let range = u128::wrapping_sub(isize::MAX as u128, isize::MIN as u128);
    let expected = range.checked_add(1);
    match expected {
        Some(value) => assert_eq!(generator.cardinality(), Some(value)),
        None => assert_eq!(generator.cardinality(), None),
    }
}

// Inverse ranges (end < start) are normalized to their forward counterpart,
// so cardinality matches sampling: both see the same [low, high] range.
#[test]
fn inverse_u8_range_has_same_cardinality_as_forward() {
    // 10u8..=0 is normalized to Range(0, 10) — identical to 0u8..=10
    assert_eq!((10u8..=0).cardinality(), Some(11));
    assert_eq!((10u8..=0).cardinality(), (0u8..=10).cardinality());
}

#[test]
fn inverse_i32_range_has_same_cardinality_as_forward() {
    assert_eq!((100i32..= -100).cardinality(), (-100i32..=100).cardinality());
}

#[test]
fn inverse_char_range_has_same_cardinality_as_forward() {
    // 'z'..='a' is normalized to Range('a', 'z') — 26 values
    assert_eq!(('z'..='a').cardinality(), Some(26));
    assert_eq!(('z'..='a').cardinality(), ('a'..='z').cardinality());
}

#[test]
fn inverse_u8_full_range_has_same_cardinality_as_forward() {
    // u8::MAX..=u8::MIN normalizes to Range(0, 255) — all 256 values
    assert_eq!((u8::MAX..=u8::MIN).cardinality(), Some(256));
    assert_eq!((u8::MAX..=u8::MIN).cardinality(), (u8::MIN..=u8::MAX).cardinality());
}
