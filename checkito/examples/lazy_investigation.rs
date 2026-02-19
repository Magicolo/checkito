//! Testing lazy cardinality more carefully

use checkito::*;
use checkito::primitive::Full;

fn main() {
    println!("=== Lazy Cardinality Investigation ===\n");
    
    // Test the static CARDINALITY
    type LazyRange = lazy::Lazy<std::ops::RangeInclusive<u8>, fn() -> std::ops::RangeInclusive<u8>>;
    println!("LazyRange::CARDINALITY: {:?}", LazyRange::CARDINALITY);
    
    // Test actual lazy instance
    fn make_range() -> std::ops::RangeInclusive<u8> { 0..=10 }
    let gen = lazy::Lazy::new(make_range);
    println!("lazy(|| 0..=10) CARDINALITY: {:?}", gen.cardinality());
    
    // Test with bool
    fn make_bool() -> Full<bool> { bool::generator() }
    let gen = lazy::Lazy::new(make_bool);
    println!("lazy(|| bool) CARDINALITY: {:?}", gen.cardinality());
    
    // Test the prelude lazy function
    let gen = lazy(|| 0u8..=10);
    println!("lazy(|| 0u8..=10) via prelude: {:?}", gen.cardinality());
}
