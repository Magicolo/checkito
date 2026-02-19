//! Testing edge cases with wrapper generators and special types

use checkito::*;

fn main() {
    println!("=== Wrapper and Special Type Cardinality Tests ===\n");
    
    test_keep();
    test_flatten();
    test_dampen();
    test_with();
    test_lazy();
    test_boxed();
    
    println!("\n=== Complete ===");
}

fn test_keep() {
    println!("--- Keep Cardinality ---");
    
    // keep should preserve cardinality
    let gen = (0u8..=10).keep();
    println!("(0u8..=10).keep():");
    println!("  Cardinality: {:?}", gen.cardinality());
    println!("  Expected: Some(11)");
    if gen.cardinality() == Some(11) {
        println!("  ✓ Correct");
    } else {
        println!("  ❌ Incorrect!");
    }
    
    println!();
}

fn test_flatten() {
    println!("--- Flatten Cardinality ---");
    
    // Flatten should multiply cardinalities
    let gen = (0u8..=1, 0u8..=1).map(|(a, b)| (a, b)).flatten();
    println!("Nested generator with flatten:");
    println!("  Cardinality: {:?}", gen.cardinality());
    // This is complex - flatten should multiply inner and outer
    
    println!();
}

fn test_dampen() {
    println!("--- Dampen Cardinality ---");
    
    // Dampen should preserve cardinality
    let gen = (0u8..=10).dampen();
    println!("(0u8..=10).dampen():");
    println!("  Cardinality: {:?}", gen.cardinality());
    println!("  Expected: Some(11) (preserves underlying cardinality)");
    if gen.cardinality() == Some(11) {
        println!("  ✓ Correct");
    } else {
        println!("  ❌ Incorrect!");
    }
    
    println!();
}

fn test_with() {
    println!("--- With Cardinality ---");
    
    // Test without with() for now - may not be available
    println!("  (Skipping with() test - may not be available)");
    
    println!();
}

fn test_lazy() {
    println!("--- Lazy Cardinality ---");
    
    // lazy should preserve cardinality
    let gen = lazy(|| 0u8..=10);
    println!("lazy(|| 0u8..=10):");
    println!("  Cardinality: {:?}", gen.cardinality());
    println!("  Expected: Some(11)");
    if gen.cardinality() == Some(11) {
        println!("  ✓ Correct");
    } else {
        println!("  ❌ Incorrect!");
    }
    
    println!();
}

fn test_boxed() {
    println!("--- Boxed Cardinality ---");
    
    // boxed should preserve cardinality
    let gen = (0u8..=10).boxed();
    println!("(0u8..=10).boxed():");
    println!("  Cardinality: {:?}", gen.cardinality());
    println!("  Expected: Some(11)");
    if gen.cardinality() == Some(11) {
        println!("  ✓ Correct");
    } else {
        println!("  ❌ Incorrect!");
    }
    
    println!();
}
