pub mod common;
use checkito::state::Weight;
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
    let set = generator
        .checks(|_| true)
        .flat_map(|result| result.into_item())
        .collect::<HashSet<_>>();

    assert_eq!(
        generator.cardinality(),
        Some((1u16..=5).len() as u128 + (10u16..=50).len() as u128 + (100u16..=500).len() as u128)
    );

    for i in 1u16..=5 {
        assert!(set.contains(&i));
    }
    for i in 10u16..=50 {
        assert!(set.contains(&i));
    }
    for i in 100u16..=500 {
        assert!(set.contains(&i));
    }
}

#[test]
fn uses_random_sampling_when_cardinality_exceeds_iterations() {
    let generator = any([1u16..=5, 10u16..=50, 100u16..=500]);
    let mut checker = generator.clone().checker();
    checker.generate.count = 8;
    checker.generate.seed = 0;
    let values = checker
        .checks(|_| true)
        .flat_map(|result| result.into_item())
        .collect::<Vec<_>>();
    let mut exhaustive = generator.checker();
    exhaustive.generate.count = 8;
    exhaustive.generate.exhaustive = Some(true);
    let exhaustive_values = exhaustive
        .checks(|_| true)
        .flat_map(|result| result.into_item())
        .collect::<Vec<_>>();

    assert_eq!(values.len(), 8);
    assert_eq!(exhaustive_values, vec![1, 2, 3, 4, 5, 10, 11, 12]);
    assert_ne!(values, exhaustive_values);
}

#[test]
fn forces_exhaustive_generation_when_requested() {
    let generator = any([1u16..=5, 10u16..=50, 100u16..=500]);
    let mut checker = generator.checker();
    checker.generate.count = 8;
    checker.generate.exhaustive = Some(true);
    let values = checker
        .checks(|_| true)
        .flat_map(|result| result.into_item())
        .collect::<Vec<_>>();

    assert_eq!(values, vec![1, 2, 3, 4, 5, 10, 11, 12]);
}

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check(100usize..=1000)]
    fn weighted_any_respects_weight_ordering_for_arbitrary_sample_count(count: usize) {
        let samples = (
            Weight::new(1.0, 1),
            Weight::new(1_000.0, 1_000),
            Weight::new(1_000_000.0, 1_000_000),
        )
            .unify::<i32>()
            .samples(count)
            .collect::<Vec<_>>();
        let one = samples.iter().filter(|&&value| value == 1).count();
        let kilo = samples.iter().filter(|&&value| value == 1_000).count();
        let mega = samples.iter().filter(|&&value| value == 1_000_000).count();
        // With weights 1:10:100 and >= 500 samples, the weak ordering holds
        // reliably. The <= comparison allows ties but not inversions.
        assert!(one <= kilo);
        assert!(kilo <= mega);
    }
}
