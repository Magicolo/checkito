//! Comprehensive experiments to test the cardinality feature
//! Testing both static CARDINALITY and dynamic cardinality() methods

use checkito::*;
use checkito::primitive::Full;
use std::ops;

fn main() {
    println!("=== Cardinality Experiments ===\n");
    
    test_primitives();
    test_ranges();
    test_boundary_cases();
    test_composites();
    test_combinators();
    test_overflow_cases();
    test_char_edge_cases();
    test_float_edge_cases();
    
    println!("\n=== All experiments complete ===");
}

fn test_primitives() {
    println!("--- Testing Primitive Types ---");
    
    // Boolean
    let gen = bool::generator();
    println!("bool CARDINALITY (static): {:?}", <Full<bool> as Generate>::CARDINALITY);
    println!("bool cardinality() (dynamic): {:?}", gen.cardinality());
    // NOTE: Found potential issue - bool value itself has CARDINALITY of 1 via same! macro
    // but Full<bool> has CARDINALITY of 2
    assert_eq!(gen.cardinality(), Some(2));
    
    // Test a specific bool value
    let gen = true;
    println!("true CARDINALITY: {:?}", <bool as Generate>::CARDINALITY);
    println!("true cardinality(): {:?}", gen.cardinality());
    // A specific bool value should have cardinality of 1
    assert_eq!(gen.cardinality(), Some(1));
    
    // u8
    let gen = u8::generator();
    println!("u8 CARDINALITY: {:?}", <ops::RangeInclusive<u8> as Generate>::CARDINALITY);
    println!("u8 cardinality(): {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(256));
    
    // i8
    let gen = i8::generator();
    println!("i8 cardinality(): {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(256));
    
    // u16
    let gen = u16::generator();
    println!("u16 cardinality(): {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(65536));
    
    // u32
    let gen = u32::generator();
    println!("u32 cardinality(): {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(4294967296));
    
    // u64
    let gen = u64::generator();
    println!("u64 cardinality(): {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(18446744073709551616));
    
    // u128 - should overflow
    let gen = u128::generator();
    println!("u128 cardinality(): {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), None);
    
    // i128 - should overflow
    let gen = i128::generator();
    println!("i128 cardinality(): {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), None);
    
    println!();
}

fn test_ranges() {
    println!("--- Testing Range Types ---");
    
    // Small range
    let gen = 0u8..=10;
    println!("0u8..=10 cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(11));
    
    // Single value range
    let gen = 42u8..=42;
    println!("42u8..=42 cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(1));
    
    // Negative range
    let gen = -10i8..=10;
    println!("-10i8..=10 cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(21));
    
    // Full i8 range
    let gen = i8::MIN..=i8::MAX;
    println!("i8::MIN..=i8::MAX cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(256));
    
    // Full u8 range
    let gen = u8::MIN..=u8::MAX;
    println!("u8::MIN..=u8::MAX cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(256));
    
    println!();
}

fn test_boundary_cases() {
    println!("--- Testing Boundary Cases ---");
    
    // Maximum u128 range that should NOT overflow
    let gen = 0u128..=0;
    println!("0u128..=0 cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(1));
    
    // Large but valid u128 range
    let gen = 0u128..=1000;
    println!("0u128..=1000 cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(1001));
    
    // Edge case: u128::MAX..=u128::MAX
    let gen = u128::MAX..=u128::MAX;
    println!("u128::MAX..=u128::MAX cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(1));
    
    // Edge case: i128::MIN..=i128::MIN
    let gen = i128::MIN..=i128::MIN;
    println!("i128::MIN..=i128::MIN cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(1));
    
    // Edge case: i128::MAX..=i128::MAX
    let gen = i128::MAX..=i128::MAX;
    println!("i128::MAX..=i128::MAX cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(1));
    
    println!();
}

fn test_composites() {
    println!("--- Testing Composite Types ---");
    
    // Option<bool> - using Some
    let gen = Some(bool::generator());
    println!("Some(bool) cardinality: {:?}", gen.cardinality());
    // This should be 1 since it's always Some
    assert_eq!(gen.cardinality(), Some(2));
    
    // Test None
    let gen: Option<ops::RangeInclusive<u8>> = None;
    println!("None::<u8 range> cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(1));
    
    // Tuple (bool, bool)
    let gen = (bool::generator(), bool::generator());
    println!("(bool, bool) cardinality: {:?}", gen.cardinality());
    // Should be 4
    assert_eq!(gen.cardinality(), Some(4));
    
    // Tuple (u8 range, bool)
    let gen = (0u8..=10, bool::generator());
    println!("(0u8..=10, bool) cardinality: {:?}", gen.cardinality());
    // Should be 11 * 2 = 22
    assert_eq!(gen.cardinality(), Some(22));
    
    // Array [bool; 3]
    let gen = [bool::generator(); 3];
    println!("[bool; 3] cardinality: {:?}", gen.cardinality());
    // Should be 2^3 = 8
    assert_eq!(gen.cardinality(), Some(8));
    
    // Tuple with u8 ranges
    let gen = (0u8..=1, 0u8..=1);
    println!("(0u8..=1, 0u8..=1) cardinality: {:?}", gen.cardinality());
    // Should be 2 * 2 = 4
    assert_eq!(gen.cardinality(), Some(4));
    
    println!();
}

fn test_combinators() {
    println!("--- Testing Combinators ---");
    
    // map doesn't change cardinality
    let gen = Generate::map(0u8..=10, |x| x * 2);
    println!("(0u8..=10).map(|x| x * 2) cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(11));
    
    // any should sum cardinalities
    let gen = (0u8..=10, 20u8..=30).any();
    println!("(0u8..=10, 20u8..=30).any() cardinality: {:?}", gen.cardinality());
    // Should be 11 + 11 = 22
    assert_eq!(gen.cardinality(), Some(22));
    
    // any with 3 choices
    let gen = (bool::generator(), 0u8..=1, 10u8..=12).any();
    println!("(bool, 0u8..=1, 10u8..=12).any() cardinality: {:?}", gen.cardinality());
    // Should be 2 + 2 + 3 = 7
    assert_eq!(gen.cardinality(), Some(7));
    
    println!();
}

fn test_overflow_cases() {
    println!("--- Testing Overflow Cases ---");
    
    // Test that large cardinalities overflow to None
    let gen = u128::generator();
    println!("u128::generator() cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), None);
    
    // Tuple that should overflow
    let gen = (u64::generator(), u64::generator());
    println!("(u64, u64) cardinality: {:?}", gen.cardinality());
    // Should overflow: 2^64 * 2^64 = 2^128
    assert_eq!(gen.cardinality(), None);
    
    // Array that should overflow
    let gen = [u64::generator(); 2];
    println!("[u64; 2] cardinality: {:?}", gen.cardinality());
    // Should overflow: 2^64 ^ 2 = 2^128
    assert_eq!(gen.cardinality(), None);
    
    // any that should overflow
    let gen = (u128::generator(), u8::generator()).any();
    println!("(u128, u8).any() cardinality: {:?}", gen.cardinality());
    // Should be None (None + Some(256) = None)
    assert_eq!(gen.cardinality(), None);
    
    println!();
}

fn test_char_edge_cases() {
    println!("--- Testing Char Edge Cases ---");
    
    // Full char range
    let gen = char::generator();
    println!("char::generator() cardinality: {:?}", gen.cardinality());
    // char has valid Unicode code points, but surrogates are invalid
    let expected = (char::MAX as u128) - (0 as u128) + 1;
    println!("Expected char cardinality (naive): {}", expected);
    println!("Actual char cardinality: {:?}", gen.cardinality());
    
    // ASCII range
    let gen = 'a'..='z';
    println!("'a'..='z' cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(26));
    
    // Single char
    let gen = 'x'..='x';
    println!("'x'..='x' cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(1));
    
    // Numbers in char
    let gen = '0'..='9';
    println!("'0'..='9' cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(10));
    
    println!();
}

fn test_float_edge_cases() {
    println!("--- Testing Float Edge Cases ---");
    
    // f32
    let gen = f32::generator();
    println!("f32::generator() cardinality: {:?}", gen.cardinality());
    // Floats should have None cardinality (infinite possible values)
    
    // f64
    let gen = f64::generator();
    println!("f64::generator() cardinality: {:?}", gen.cardinality());
    // Floats should have None cardinality (infinite possible values)
    
    // Range of f32
    let gen = 0.0f32..=1.0;
    println!("0.0f32..=1.0 cardinality: {:?}", gen.cardinality());
    
    // Range of f64
    let gen = -1.0f64..=1.0;
    println!("-1.0f64..=1.0 cardinality: {:?}", gen.cardinality());
    
    println!();
}
