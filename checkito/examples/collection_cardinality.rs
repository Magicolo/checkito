//! Testing cardinality with collections and dynamic sizes

use checkito::*;

fn main() {
    println!("=== Collection Cardinality Tests ===\n");
    
    test_vec_cardinality();
    test_collect_cardinality();
    test_nested_collections();
    test_filter_cardinality();
    
    println!("\n=== Complete ===");
}

fn test_vec_cardinality() {
    println!("--- Vec Cardinality ---");
    
    // Vec of fixed size
    let gen = Generate::collect_with::<_, Vec<_>>(0u8..=1, 2..=2);
    println!("Vec<0u8..=1> with length 2..=2:");
    println!("  Cardinality: {:?}", gen.cardinality());
    // Should be 2^2 = 4
    let expected = 2u128.pow(2);
    println!("  Expected: {}", expected);
    if gen.cardinality() == Some(expected) {
        println!("  ✓ Correct");
    } else {
        println!("  ❌ Incorrect!");
    }
    
    // Vec with range of sizes
    let gen = Generate::collect_with::<_, Vec<_>>(bool::generator(), 0..=2);
    println!("\nVec<bool> with length 0..=2:");
    println!("  Cardinality: {:?}", gen.cardinality());
    // Should be: 1 (empty) + 2 (len=1) + 4 (len=2) = 7
    let expected = 1 + 2 + 4;
    println!("  Expected: {} (1 + 2 + 4)", expected);
    if gen.cardinality() == Some(expected) {
        println!("  ✓ Correct");
    } else {
        println!("  ❌ Incorrect!");
    }
    
    // Vec with larger range
    let gen = Generate::collect_with::<_, Vec<_>>(0u8..=1, 0..=3);
    println!("\nVec<0u8..=1> with length 0..=3:");
    println!("  Cardinality: {:?}", gen.cardinality());
    // Should be: 1 + 2 + 4 + 8 = 15
    let expected = 1 + 2 + 4 + 8;
    println!("  Expected: {} (1 + 2 + 4 + 8)", expected);
    
    println!();
}

fn test_collect_cardinality() {
    println!("--- Collect Cardinality ---");
    
    // String collection
    let gen = Generate::collect_with::<_, String>('a'..='z', 3..=3);
    println!("String of 3 lowercase letters:");
    println!("  Cardinality: {:?}", gen.cardinality());
    // Should be 26^3 = 17576
    let expected = 26u128.pow(3);
    println!("  Expected: {}", expected);
    if gen.cardinality() == Some(expected) {
        println!("  ✓ Correct");
    } else {
        println!("  ❌ Incorrect!");
    }
    
    // String with variable length
    let gen = Generate::collect_with::<_, String>('a'..='b', 0..=2);
    println!("\nString of 0-2 chars from 'a'..='b':");
    println!("  Cardinality: {:?}", gen.cardinality());
    // Should be: 1 (empty) + 2 (len=1) + 4 (len=2) = 7
    let expected = 1 + 2 + 4;
    println!("  Expected: {} (1 + 2 + 4)", expected);
    
    println!();
}

fn test_nested_collections() {
    println!("--- Nested Collections ---");
    
    // Vec of Vec
    let inner = Generate::collect_with::<_, Vec<_>>(bool::generator(), 1..=1);
    let gen = Generate::collect_with::<_, Vec<_>>(inner, 2..=2);
    println!("Vec<Vec<bool>> where each inner has len 1, outer has len 2:");
    println!("  Cardinality: {:?}", gen.cardinality());
    // Inner vec has cardinality 2 (just [true] or [false])
    // Outer vec has 2 of those, so 2^2 = 4
    let expected = 2u128.pow(2);
    println!("  Expected: {}", expected);
    
    println!();
}

fn test_filter_cardinality() {
    println!("--- Filter Cardinality ---");
    
    // Filter should return None for cardinality since we can't know
    // how many values will pass the filter
    let gen = Generate::filter(0u8..=10, |x| x % 2 == 0);
    println!("(0u8..=10).filter(even):");
    println!("  Cardinality: {:?}", gen.cardinality());
    println!("  Expected: None (can't determine statically)");
    if gen.cardinality().is_none() {
        println!("  ✓ Correct");
    } else {
        println!("  ❌ Should be None!");
    }
    
    println!();
}
