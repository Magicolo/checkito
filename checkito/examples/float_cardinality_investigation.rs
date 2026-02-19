//! Deep dive into float cardinality

use checkito::*;

fn main() {
    println!("=== Float Cardinality Investigation ===\n");
    
    test_f32_cardinality();
    test_f64_cardinality();
    test_special_floats();
    test_float_ranges();
    
    println!("\n=== Complete ===");
}

fn test_f32_cardinality() {
    println!("--- f32 Cardinality ---");
    
    let gen = f32::generator();
    let card = gen.cardinality();
    println!("f32::generator() cardinality: {:?}", card);
    
    // f32 has 2^32 bit patterns
    let expected_bit_patterns = 2u128.pow(32);
    println!("Expected bit patterns: {}", expected_bit_patterns);
    
    if let Some(c) = card {
        println!("Actual: {}", c);
        if c == expected_bit_patterns {
            println!("✓ Matches bit patterns");
        } else {
            println!("❌ Doesn't match expected bit patterns");
            println!("   Difference: {}", (c as i128) - (expected_bit_patterns as i128));
        }
    }
    
    println!();
}

fn test_f64_cardinality() {
    println!("--- f64 Cardinality ---");
    
    let gen = f64::generator();
    let card = gen.cardinality();
    println!("f64::generator() cardinality: {:?}", card);
    
    // f64 has 2^64 bit patterns
    let expected_bit_patterns = 2u128.pow(64);
    println!("Expected bit patterns: {}", expected_bit_patterns);
    
    if let Some(c) = card {
        println!("Actual: {}", c);
        if c == expected_bit_patterns {
            println!("✓ Matches bit patterns");
        } else {
            println!("❌ Doesn't match expected bit patterns");
            println!("   Difference: {}", (c as i128) - (expected_bit_patterns as i128));
        }
    }
    
    println!();
}

fn test_special_floats() {
    println!("--- Special Float Values ---");
    
    // Are NaN, infinity, etc. counted?
    println!("f32 special values:");
    println!("  NaN: {}", f32::NAN);
    println!("  INFINITY: {}", f32::INFINITY);
    println!("  NEG_INFINITY: {}", f32::NEG_INFINITY);
    println!("  Note: NaN has multiple bit patterns!");
    
    // f32 has:
    // - 1 bit sign
    // - 8 bits exponent
    // - 23 bits mantissa
    // Total: 32 bits = 2^32 patterns
    //
    // But not all are valid numbers:
    // - Multiple NaN representations (exponent all 1s, mantissa non-zero)
    // - Special handling for denormals
    
    println!("\nDo generators produce NaN and infinity?");
    let samples: Vec<f32> = f32::generator().samples(1000).collect();
    let has_nan = samples.iter().any(|x| x.is_nan());
    let has_inf = samples.iter().any(|x| x.is_infinite());
    let has_neg_inf = samples.iter().any(|x| *x == f32::NEG_INFINITY);
    
    println!("  Contains NaN: {}", has_nan);
    println!("  Contains +∞: {}", has_inf);
    println!("  Contains -∞: {}", has_neg_inf);
    
    println!();
}

fn test_float_ranges() {
    println!("--- Float Ranges ---");
    
    // Small range
    let gen = 0.0f32..=1.0;
    println!("0.0f32..=1.0 cardinality: {:?}", gen.cardinality());
    
    // How many f32 values between 0 and 1?
    // This is not infinite - there are discrete bit patterns
    let card = gen.cardinality();
    if let Some(c) = card {
        println!("  Actual: {} discrete values", c);
    }
    
    // Negative range
    let gen = -1.0f32..=1.0;
    println!("-1.0f32..=1.0 cardinality: {:?}", gen.cardinality());
    
    // Very small range
    let gen = 0.0f32..=0.0;
    println!("0.0f32..=0.0 cardinality: {:?}", gen.cardinality());
    // Should this be 1 (just zero) or 2 (zero and negative zero)?
    
    // Check if generator distinguishes +0.0 and -0.0
    println!("\nDoes 0.0f32 == -0.0f32? {}", 0.0f32 == -0.0f32);
    println!("Do they have same bits? {}", 
             0.0f32.to_bits() == (-0.0f32).to_bits());
    
    println!();
}
