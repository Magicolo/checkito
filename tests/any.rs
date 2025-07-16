pub mod common;
use checkito::any::Weight;
use common::*;
use std::collections::HashSet;

#[test]
fn weighted_any() {
    let samples = (
        Weight::new(1.0, 1),
        Weight::new(10.0, 10),
        Weight::new(100.0, 100),
    )
        .unify::<i32>()
        .samples(1000)
        .collect::<Vec<_>>();
    let one = samples.iter().filter(|&&value| value == 1).count();
    let ten = samples.iter().filter(|&&value| value == 10).count();
    let hundred = samples.iter().filter(|&&value| value == 100).count();
    assert!(one < ten);
    assert!(ten < hundred);
}

#[test]
fn generates_exhaustively() {
    let generator = &any([1u16..=5, 10u16..=50, 100u16..=500]);
    let set = dbg!(
        any([1u16..=5, 10u16..=50, 100u16..=500])
            .checks(|_| true)
            .flat_map(|result| result.item())
            .collect::<HashSet<_>>()
    );

    assert_eq!(
        generator.cardinality(),
        Some((1u16..=5).len() as u128 + (10u16..=50).len() as u128 + (100u16..=500).len() as u128)
    );

    for i in 0u16..=5 {
        assert!(set.contains(&i));
    }
    for i in 10u16..=50 {
        assert!(set.contains(&i));
    }
    for i in 100u16..=500 {
        assert!(set.contains(&i));
    }
}
