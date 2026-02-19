//! Testing any() combinator cardinality edge cases

use checkito::*;

fn main() {
    println!("=== Any Combinator Cardinality Tests ===\n");
    
    test_any_basic();
    test_any_with_none();
    test_any_overflow();
    test_any_nested();
    test_unify();
    
    println!("\n=== Complete ===");
}

fn test_any_basic() {
    println!("--- Basic Any ---");
    
    // Two choices
    let gen = (0u8..=10, 20u8..=30).any();
    println!("(0..=10, 20..=30).any() cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(22), "11 + 11 = 22");
    
    // Three choices
    let gen = (0u8..=10, 20u8..=30, 40u8..=50).any();
    println!("(0..=10, 20..=30, 40..=50).any() cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(33), "11 + 11 + 11 = 33");
    
    // Different cardinalities
    let gen = (bool::generator(), 0u8..=10).any();
    println!("(bool, 0..=10).any() cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(13), "2 + 11 = 13");
    
    println!();
}

fn test_any_with_none() {
    println!("--- Any with None Cardinality ---");
    
    // One choice has None cardinality
    let gen = (u128::generator(), 0u8..=10).any();
    println!("(u128, 0..=10).any() cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), None, "None + 11 = None");
    
    // Both have None
    let gen = (u128::generator(), String::generator()).any();
    println!("(u128, String).any() cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), None, "None + None = None");
    
    println!();
}

fn test_any_overflow() {
    println!("--- Any Overflow ---");
    
    // Sum that would overflow
    let gen = (u64::generator(), u64::generator()).any();
    println!("(u64, u64).any() cardinality: {:?}", gen.cardinality());
    // 2^64 + 2^64 = 2 * 2^64 = 2^65
    let expected = (u64::MAX as u128 + 1) * 2;
    println!("  Expected: {}", expected);
    
    // Large sum near u128::MAX
    let max_half = u128::MAX / 2;
    let gen = (0u128..=max_half, 0u128..=max_half).any();
    println!("Two ranges totaling near u128::MAX:");
    println!("  Cardinality: {:?}", gen.cardinality());
    
    // Sum that overflows u128
    let gen = (0u128..=u128::MAX - 1, 0u128..=u128::MAX - 1).any();
    println!("Sum that should overflow:");
    println!("  Cardinality: {:?}", gen.cardinality());
    // (u128::MAX) + (u128::MAX) would overflow
    
    println!();
}

fn test_any_nested() {
    println!("--- Nested Any ---");
    
    // Nested any should flatten the sum
    let gen = ((0u8..=5, 6u8..=10).any(), 11u8..=15).any();
    println!("((0..=5, 6..=10).any(), 11..=15).any():");
    println!("  Cardinality: {:?}", gen.cardinality());
    // (6 + 5) + 5 = 16? Or does it preserve structure?
    
    println!();
}

fn test_unify() {
    println!("--- Unify with Any ---");
    
    // unify should preserve cardinality
    let gen = (0u8..=10, 20u8..=30).any().unify::<u8>();
    println!("(0..=10, 20..=30).any().unify::<u8>():");
    println!("  Cardinality: {:?}", gen.cardinality());
    assert_eq!(gen.cardinality(), Some(22), "Unify preserves cardinality");
    
    // But are the values actually unified?
    let samples: Vec<u8> = gen.samples(100).collect();
    println!("  Sample values: {:?}", &samples[..10.min(samples.len())]);
    
    println!();
}
