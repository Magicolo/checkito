//! Testing mathematical edge cases in cardinality calculations

use checkito::*;

fn main() {
    println!("=== Mathematical Edge Cases ===\n");
    
    test_overflow_in_product();
    test_overflow_in_sum();
    test_overflow_in_power();
    test_zero_cardinality();
    test_one_cardinality();
    
    println!("\n=== Complete ===");
}

fn test_overflow_in_product() {
    println!("--- Product Overflow ---");
    
    // Two large types multiplied together should overflow
    let gen = (u64::generator(), u64::generator());
    println!("(u64, u64) cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), None, "Should overflow: 2^64 * 2^64 = 2^128");
    
    // Just under overflow
    let gen = (u32::generator(), u32::generator());
    println!("(u32, u32) cardinality: {:?}", gen.cardinality());
    let expected = (u32::MAX as u128 + 1) * (u32::MAX as u128 + 1);
    println!("  Expected: {}", expected);
    
    // Tuple of 3 u32s should overflow
    let gen = (u32::generator(), u32::generator(), u32::generator());
    println!("(u32, u32, u32) cardinality: {:?}", gen.cardinality());
    // 2^32 * 2^32 * 2^32 = 2^96, should fit in u128
    
    // Tuple of 4 u32s should overflow
    let gen = (u32::generator(), u32::generator(), u32::generator(), u32::generator());
    println!("(u32, u32, u32, u32) cardinality: {:?}", gen.cardinality());
    // 2^32 * 2^32 * 2^32 * 2^32 = 2^128, should overflow
    
    println!();
}

fn test_overflow_in_sum() {
    println!("--- Sum Overflow ---");
    
    // any() sums cardinalities
    let gen = (u128::MAX..=u128::MAX, u128::MAX..=u128::MAX).any();
    println!("(u128::MAX, u128::MAX).any() cardinality: {:?}", gen.cardinality());
    // 1 + 1 = 2, should work
    
    // Large sums
    let gen = (u64::generator(), u64::generator()).any();
    println!("(u64, u64).any() cardinality: {:?}", gen.cardinality());
    // 2^64 + 2^64 = 2^65, should fit in u128
    
    // Sum that overflows
    let gen = (u128::generator(), 0u8..=10).any();
    println!("(u128, u8 range).any() cardinality: {:?}", gen.cardinality());
    // None + 11 = None
    assert_eq!(gen.cardinality(), None);
    
    println!();
}

fn test_overflow_in_power() {
    println!("--- Power Overflow ---");
    
    // Array is power: element_cardinality ^ array_length
    let gen = [u8::generator(); 2];
    println!("[u8; 2] cardinality: {:?}", gen.cardinality());
    // 256^2 = 65536
    
    let gen = [u8::generator(); 3];
    println!("[u8; 3] cardinality: {:?}", gen.cardinality());
    // 256^3 = 16,777,216
    
    let gen = [u8::generator(); 16];
    println!("[u8; 16] cardinality: {:?}", gen.cardinality());
    // 256^16 = 2^128, should overflow
    
    let gen = [u8::generator(); 17];
    println!("[u8; 17] cardinality: {:?}", gen.cardinality());
    // Should definitely overflow
    assert_eq!(gen.cardinality(), None, "256^17 should overflow u128");
    
    println!();
}

fn test_zero_cardinality() {
    println!("--- Zero Cardinality ---");
    
    // Empty range
    let gen = 10u8..=0;  // Invalid range
    println!("10u8..=0 (invalid range) cardinality: {:?}", gen.cardinality());
    // What happens with an empty/invalid range?
    
    println!();
}

fn test_one_cardinality() {
    println!("--- One Cardinality ---");
    
    // Single value
    let gen = 42u8..=42;
    println!("42u8..=42 cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(1));
    
    // Constant
    let gen = true;
    println!("true cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(1));
    
    // Product with 1
    let gen = (42u8..=42, 0u8..=10);
    println!("(42u8..=42, 0u8..=10) cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(11), "1 * 11 = 11");
    
    println!();
}
