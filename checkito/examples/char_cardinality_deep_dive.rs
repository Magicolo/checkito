//! Deep dive into char cardinality issues
//! Specifically testing surrogate code points

use checkito::*;

fn main() {
    println!("=== Char Cardinality Deep Dive ===\n");
    
    test_char_basics();
    test_surrogate_ranges();
    test_actual_generation();
    
    println!("\n=== Complete ===");
}

fn test_char_basics() {
    println!("--- Char Basics ---");
    
    let gen = char::generator();
    println!("char::generator() cardinality: {:?}", gen.cardinality());
    
    // Full Unicode range
    let total_code_points = (char::MAX as u32) + 1;
    println!("Total Unicode code points (0 to char::MAX): {}", total_code_points);
    
    // Surrogate range
    let surrogate_start = 0xD800u32;
    let surrogate_end = 0xDFFFu32;
    let surrogate_count = surrogate_end - surrogate_start + 1;
    println!("Surrogate code points (U+D800 to U+DFFF): {}", surrogate_count);
    
    // Expected valid chars
    let expected_valid = total_code_points - surrogate_count;
    println!("Expected valid char count: {}", expected_valid);
    
    // Compare with actual cardinality
    let actual_cardinality = gen.cardinality().unwrap();
    println!("Actual cardinality: {}", actual_cardinality);
    
    if actual_cardinality == total_code_points as u128 {
        println!("❌ ISSUE: Cardinality includes surrogate code points!");
    } else if actual_cardinality == expected_valid as u128 {
        println!("✓ Cardinality correctly excludes surrogates");
    } else {
        println!("⚠ Unexpected cardinality value");
    }
    
    println!();
}

fn test_surrogate_ranges() {
    println!("--- Surrogate Ranges ---");
    
    // Range that includes surrogates
    let surrogate_start = char::from_u32(0xD7FF).unwrap();
    let surrogate_end = char::from_u32(0xE000).unwrap();
    
    println!("Range from U+D7FF to U+E000:");
    println!("  Start: {:?} (U+{:04X})", surrogate_start, surrogate_start as u32);
    println!("  End: {:?} (U+{:04X})", surrogate_end, surrogate_end as u32);
    
    let gen = surrogate_start..=surrogate_end;
    println!("  Cardinality: {:?}", gen.cardinality());
    
    // What should the cardinality be?
    // From D7FF to E000 is: E000 - D7FF + 1 = 513
    // But D800-DFFF (2048 values) are invalid surrogates
    // So valid chars: 1 (D7FF) + 1 (E000) = 2
    // But the range calculation doesn't know about surrogates!
    
    let naive_count = (surrogate_end as u32) - (surrogate_start as u32) + 1;
    println!("  Naive count (end - start + 1): {}", naive_count);
    
    println!();
}

fn test_actual_generation() {
    println!("--- Actual Generation Test ---");
    
    // Generate some chars and see if we ever get invalid surrogates
    let gen = char::generator();
    
    println!("Generating 1000 random chars...");
    let mut samples: Vec<char> = gen.samples(1000).collect();
    samples.sort();
    samples.dedup();
    
    let mut has_surrogates = false;
    for c in &samples {
        let code_point = *c as u32;
        if code_point >= 0xD800 && code_point <= 0xDFFF {
            println!("❌ Generated surrogate: {:?} (U+{:04X})", c, code_point);
            has_surrogates = true;
        }
    }
    
    if !has_surrogates {
        println!("✓ No surrogates generated in sample");
    }
    
    println!("Unique chars generated: {}", samples.len());
    
    println!();
}
