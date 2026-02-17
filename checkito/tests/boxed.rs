pub mod common;
use common::*;

#[test]
fn boxed_generator_supports_sampling_and_cardinality() {
    let generator = boxed(Box::new(0u8..=2));

    assert_eq!(generator.cardinality(), Some(3));

    let values = generator
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();
    assert_eq!(values, vec![0, 1, 2]);
}

#[test]
fn boxed_downcast_succeeds_for_original_type() {
    let generator = boxed(Box::new(0u8..=2));
    let original = generator
        .downcast::<std::ops::RangeInclusive<u8>>()
        .unwrap();

    assert_eq!(original, Box::new(0u8..=2));
}

#[test]
fn boxed_downcast_failure_returns_original_generator() {
    let generator = boxed(Box::new(0u8..=2));
    let boxed = generator
        .downcast::<std::ops::RangeInclusive<u16>>()
        .unwrap_err();

    assert_eq!(boxed.cardinality(), Some(3));
    assert!(boxed.samples(32).all(|value| value <= 2));
}

#[test]
fn boxed_shrinker_can_clone_shrink_and_failed_downcast_preserves_behavior() {
    let mut shrinker = shrinker(boxed(Box::new(1u8..=4))).sample(1.0);

    let item = shrinker.item();
    let clone = shrinker.clone();
    assert_eq!(item, clone.item());

    let wrong = clone.downcast::<same::Same<u8>>().unwrap_err();
    assert_eq!(wrong.item(), item);

    if let Some(shrunk) = shrinker.shrink() {
        assert!(shrunk.item() <= item);
    } else {
        assert_eq!(item, 1);
    }
}

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check(0u8..=10, 0u8..=10)]
    fn downcast_failure_preserves_generated_range(start: u8, end: u8) {
        let (low, high) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let generator = boxed(Box::new(low..=high));
        let boxed = generator
            .downcast::<std::ops::RangeInclusive<u16>>()
            .unwrap_err();

        assert_eq!(boxed.cardinality(), Some(u128::from(high - low) + 1));
        assert!(boxed.samples(32).all(|value| value >= low && value <= high));
    }
}
