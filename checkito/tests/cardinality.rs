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
    // char excludes surrogate code points U+D800..=U+DFFF (2,048 values)
    // 1,114,112 total - 2,048 surrogates = 1,112,064
    assert_eq!(generator.cardinality(), Some(1_112_064));
}

#[test]
fn char_surrogate_spanning_range_has_correct_cardinality() {
    // Range spanning the surrogate gap: U+D7FF and U+E000 are valid, surrogates are
    // not
    let generator = '\u{D7FF}'..='\u{E000}';
    // 0xE000 - 0xD7FF + 1 = 2050 total code points, minus 2048 surrogates = 2
    assert_eq!(generator.cardinality(), Some(2));
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
fn f32_full_range_has_correct_cardinality() {
    let generator = f32::generator();
    // All non-NaN values (finite + both infinities) + 1 for NaN (all NaN patterns
    // treated as one).
    assert_eq!(generator.cardinality(), Some(4278190083));
}

#[test]
fn f64_full_range_has_correct_cardinality() {
    let generator = f64::generator();
    // All non-NaN values (finite + both infinities) + 1 for NaN (all NaN patterns
    // treated as one).
    assert_eq!(generator.cardinality(), Some(18437736874454810627));
}

#[test]
fn f32_full_cardinality_equals_finite_plus_infinities_plus_nan() {
    // The full f32 cardinality is: all finite values (MIN..=MAX) + NEG_INFINITY +
    // INFINITY + NaN. The generator covers all three: finite via range
    // branches, ±INF and NaN via the Special branch.
    let finite = (f32::MIN..=f32::MAX).cardinality().unwrap();
    assert_eq!(f32::generator().cardinality(), Some(finite + 3));
}

#[test]
fn f64_full_cardinality_equals_finite_plus_infinities_plus_nan() {
    // The full f64 cardinality is: all finite values (MIN..=MAX) + NEG_INFINITY +
    // INFINITY + NaN. The generator covers all three: finite via range
    // branches, ±INF and NaN via the Special branch.
    let finite = (f64::MIN..=f64::MAX).cardinality().unwrap();
    assert_eq!(f64::generator().cardinality(), Some(finite + 3));
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

#[test]
fn lazy_delegates_cardinality_to_inner_range() {
    let generator = lazy(|| 0u8..=10);
    assert_eq!(generator.cardinality(), Some(256));
    generator.sample(1.0);
    assert_eq!(generator.cardinality(), Some(11));
}

#[test]
fn lazy_delegates_cardinality_to_inner_bool() {
    let generator = lazy(|| bool::generator());
    assert_eq!(generator.cardinality(), Some(2));
    generator.sample(1.0);
    assert_eq!(generator.cardinality(), Some(2));
}

#[test]
fn lazy_delegates_cardinality_to_inner_unbounded() {
    let generator = lazy(|| u128::generator());
    assert_eq!(generator.cardinality(), None);
    generator.sample(1.0);
    assert_eq!(generator.cardinality(), None);
}
