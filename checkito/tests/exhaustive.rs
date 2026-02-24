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
#[allow(clippy::reversed_empty_ranges)]
fn inverse_range_is_normalized_and_exhaustive_covers_same_values() {
    // Inverse range 9u8..=0 is normalized to Range(0, 9) at conversion time,
    // so exhaustive iteration produces the same values as the forward range.
    let forward = (0u8..=9)
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();
    #[allow(clippy::reversed_empty_ranges)]
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

#[test]
fn full_integer_exhaustive_covers_special_values() {
    // In exhaustive mode, Full<i8> should cover its full range including
    // special values (0, MIN, MAX) through the cycling mechanism.
    let mut checker = <i8>::generator().checker();
    // Use a count large enough to cycle through range + special branches.
    // i8 range has 256 values; special has 3. Total cycle = 259.
    checker.generate.count = 259;
    checker.generate.exhaustive = Some(true);

    let values: HashSet<i8> = checker
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect();

    assert!(values.contains(&0i8));
    assert!(values.contains(&i8::MIN));
    assert!(values.contains(&i8::MAX));
}

#[test]
fn option_bool_exhaustive_covers_none_and_some() {
    // Option<bool> in exhaustive mode should deterministically produce
    // both None and Some(_) values.
    let values: Vec<Option<bool>> = Option::<bool>::generator()
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect();

    // cardinality of Option<bool> is Some(3): None, Some(false), Some(true).
    assert_eq!(values.len(), 3);
    assert!(values.contains(&None));
    assert!(values.contains(&Some(false)));
    assert!(values.contains(&Some(true)));
}

#[test]
fn result_bool_bool_exhaustive_covers_all_variants() {
    // Result<bool, bool> in exhaustive mode should cover all 4 combinations.
    let values: HashSet<Result<bool, bool>> = Result::<bool, bool>::generator()
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect();

    assert_eq!(values.len(), 4);
    assert!(values.contains(&Ok(false)));
    assert!(values.contains(&Ok(true)));
    assert!(values.contains(&Err(false)));
    assert!(values.contains(&Err(true)));
}

#[test]
fn any_tuple_exhaustive_covers_all_sub_generators() {
    // (0u8..=1, 10u8..=11).any() in exhaustive mode with count=4 should
    // produce all four values deterministically.
    let values = (0u8..=1, 10u8..=11)
        .any()
        .unify::<u8>()
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<HashSet<_>>();

    assert_eq!(values.len(), 4);
    assert!(values.contains(&0u8));
    assert!(values.contains(&1u8));
    assert!(values.contains(&10u8));
    assert!(values.contains(&11u8));
}

#[test]
fn filter_exhaustive_produces_one_value_per_index() {
    // In exhaustive mode, filter should not retry but produce one value
    // per exhaustive index (Some if accepted, None if rejected).
    let values = Generate::filter(0u8..=3, |&x| x % 2 == 0)
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();

    // 0u8..=3 has cardinality 4, so 4 exhaustive iterations.
    // Indices 0,2 produce even values (Some), indices 1,3 produce odd (None).
    assert_eq!(values.len(), 4);
    assert!(values.contains(&Some(0)));
    assert!(values.contains(&Some(2)));
    assert!(values.contains(&None));
}

#[test]
fn weighted_any_tuple_exhaustive_ignores_weights() {
    // Weighted tuple any in exhaustive mode should ignore weights and
    // deterministically cover all sub-generators.
    let values = (
        Weight::new(0.1, 0u8..=1),
        Weight::new(10.0, 10u8..=11),
    )
        .unify::<u8>()
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<HashSet<_>>();

    assert_eq!(values.len(), 4);
}
