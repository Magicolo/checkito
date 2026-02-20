pub mod common;
use checkito::{Generate, state::Weight};
use common::*;
use std::collections::HashSet;

#[derive(Clone)]
struct UnknownCardinality<G>(G);

impl<G: Generate> Generate for UnknownCardinality<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    const CARDINALITY: Option<u128> = None;

    fn generate(&self, state: &mut checkito::state::State) -> Self::Shrink {
        self.0.generate(state)
    }

    fn cardinality(&self) -> Option<u128> {
        None
    }
}

#[test]
fn range_auto_switches_to_exhaustive() {
    let values = (0u8..=9)
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();

    assert_eq!(values, Iterator::collect::<Vec<_>>(0u8..=9));
}

#[test]
fn inverse_range_is_normalized_and_exhaustive_covers_same_values() {
    // Inverse range 9u8..=0 is normalized to Range(0, 9) at conversion time,
    // so exhaustive iteration produces the same values as the forward range.
    let forward = (0u8..=9)
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();
    let inverse = (9u8..=0)
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();

    assert_eq!(inverse, forward);
}

#[test]
fn range_can_be_forced_to_random_even_if_finite() {
    let mut checker = (0u8..=9).checker();
    checker.generate.count = 25;
    checker.generate.seed = 0;
    checker.generate.exhaustive = Some(false);
    let values = checker
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();
    let exhaustive_prefix = (0u8..=9).cycle().take(25).collect::<Vec<_>>();

    assert_eq!(values.len(), 25);
    assert_ne!(values, exhaustive_prefix);
}

#[test]
fn any_slice_exhaustive_covers_all_segments() {
    let values = any([0u8..=2, 10u8..=11, 20u8..=23])
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();

    assert_eq!(
        values,
        vec![
            Some(0),
            Some(1),
            Some(2),
            Some(10),
            Some(11),
            Some(20),
            Some(21),
            Some(22),
            Some(23),
        ]
    );
}

#[test]
fn weighted_any_exhaustive_ignores_weights_and_still_covers_all() {
    let values = [
        Weight::new(0.1, 1u8..=2),
        Weight::new(1.0, 10u8..=12),
        Weight::new(10.0, 20u8..=21),
    ]
    .checks(Ok::<_, ()>)
    .map(|result| result.into_item())
    .collect::<Vec<_>>();

    assert_eq!(
        values,
        vec![
            Some(1),
            Some(2),
            Some(10),
            Some(11),
            Some(12),
            Some(20),
            Some(21),
        ]
    );
}

#[test]
fn collect_with_finite_length_is_exhaustive() {
    let values = ('a'..='b')
        .collect_with::<_, String>(0usize..=2)
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<HashSet<_>>();

    let expected = ["", "a", "b", "aa", "ab", "ba", "bb"]
        .into_iter()
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    assert_eq!(values, expected);
}

#[test]
fn forcing_exhaustive_respects_iteration_count() {
    let mut checker = ('a'..='b').collect_with::<_, String>(0usize..=2).checker();
    checker.generate.count = 1;
    checker.generate.exhaustive = Some(true);

    let values = checker
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();

    assert_eq!(values, vec!["".to_owned()]);
}

#[test]
fn repeat_with_cardinality_one_selects_each_length_once() {
    let lengths = same('x')
        .collect_with::<_, String>(3usize..=5)
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item().len())
        .collect::<Vec<_>>();

    assert_eq!(lengths, vec![3, 4, 5]);
}

#[test]
fn repeat_cardinality_one_keeps_index_for_sibling_generators() {
    let values = (same('x').collect_with::<_, String>(1usize..=3), 0u8..=2)
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();

    assert_eq!(
        values,
        vec![
            ("x".to_owned(), 0),
            ("xx".to_owned(), 0),
            ("xxx".to_owned(), 0),
            ("x".to_owned(), 1),
            ("xx".to_owned(), 1),
            ("xxx".to_owned(), 1),
            ("x".to_owned(), 2),
            ("xx".to_owned(), 2),
            ("xxx".to_owned(), 2),
        ]
    );
}

#[test]
fn repeat_with_cardinality_two_uses_geometric_length_buckets() {
    let lengths = ('a'..='b')
        .collect_with::<_, String>(2usize..=4)
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item().len())
        .collect::<Vec<_>>();

    let expected = [2usize, 3, 4]
        .into_iter()
        .flat_map(|length| std::iter::repeat_n(length, 1usize << length))
        .collect::<Vec<_>>();
    assert_eq!(lengths, expected);
}

#[test]
fn repeat_with_overflowing_initial_block_falls_back_to_minimum_length() {
    let mut checker = ('a'..='b')
        .collect_with::<_, String>(130usize..=132)
        .checker();
    checker.generate.count = 1;
    checker.generate.exhaustive = Some(true);

    let values = checker
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();

    assert_eq!(values.len(), 1);
    assert_eq!(values[0].len(), 130);
}

#[test]
fn repeat_overflow_fallback_has_deterministic_sibling_projection() {
    let mut checker = (
        ('a'..='b').collect_with::<_, String>(130usize..=132),
        0u8..=4,
    )
        .checker();
    checker.generate.count = 5;
    checker.generate.exhaustive = Some(true);

    let values = checker
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item().1)
        .collect::<Vec<_>>();

    assert_eq!(values, vec![0, 0, 0, 0, 0]);
}

#[test]
fn repeat_with_zero_cardinality_and_positive_minimum_uses_minimum_length() {
    let mut checker = any([] as [std::ops::RangeInclusive<char>; 0])
        .collect_with::<_, Vec<_>>(1usize..=3)
        .checker();
    checker.generate.count = 4;
    checker.generate.exhaustive = Some(true);

    let values = checker
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();

    assert_eq!(values, vec![vec![None], vec![None], vec![None], vec![None]]);
}

#[test]
fn repeat_with_unknown_cardinality_uses_repeat_range_generation_path() {
    let mut checker = UnknownCardinality(same('x'))
        .collect_with::<_, String>(1usize..=3)
        .checker();
    checker.generate.count = 5;
    checker.generate.exhaustive = Some(true);

    let values = checker
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();

    let lengths = values.iter().map(String::len).collect::<Vec<_>>();
    assert_eq!(lengths, vec![1, 2, 3, 1, 2]);
}

#[test]
fn convert_preserves_values_exhaustively() {
    let values = (0u8..=4)
        .convert::<u16>()
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();

    assert_eq!(values, vec![0, 1, 2, 3, 4]);
}
