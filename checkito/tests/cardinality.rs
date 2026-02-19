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
    // u128 has 2^128 values, which overflows u128
    // wrapping_add should handle this correctly
    let expected = u128::wrapping_sub(u128::MAX, u128::MIN).wrapping_add(1);
    assert_eq!(generator.cardinality(), Some(expected));
}

#[test]
fn i128_full_range_has_correct_cardinality() {
    let generator = i128::generator();
    // i128 has 2^128 values, which overflows u128
    let expected = u128::wrapping_sub(i128::MAX as u128, i128::MIN as u128).wrapping_add(1);
    assert_eq!(generator.cardinality(), Some(expected));
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
    let expected = u128::wrapping_sub(usize::MAX as u128, usize::MIN as u128).wrapping_add(1);
    assert_eq!(generator.cardinality(), Some(expected));
}

#[test]
fn full_isize_has_correct_cardinality() {
    let generator = isize::generator();
    let expected = u128::wrapping_sub(isize::MAX as u128, isize::MIN as u128).wrapping_add(1);
    assert_eq!(generator.cardinality(), Some(expected));
}
