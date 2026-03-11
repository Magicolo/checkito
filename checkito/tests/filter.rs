pub mod common;
use common::*;

#[test]
fn filtered_pair_preserves_inequality() {
    assert!(
        <(String, String)>::generator()
            .filter(|(left, right)| left != right)
            .check(|pair| match pair {
                Some((left, right)) => left != right,
                None => true,
            })
            .is_none()
    );
}

#[test]
fn filtered_array_preserves_inequality() {
    assert!(
        Generate::collect::<String>('a'..='z')
            .array::<3>()
            .filter(|[a, b, c]| a != b && b != c && a != c)
            .check(|array| match array {
                Some([a, b, c]) => a != b && b != c && a != c,
                None => true,
            })
            .is_none()
    );
}

#[test]
fn shrinked_filter_preserves_inequality() {
    let fail = (
        <(String, String)>::generator().filter(|(left, right)| left != right),
        <usize>::generator(),
    )
        .check(|(pair, value)| {
            let Some((left, right)) = pair else {
                return true;
            };
            assert_ne!(left, right);
            value < 1000 // Force the check to fail at some point.
        })
        .unwrap();
    assert_eq!(fail.cause, Cause::Disprove(()));
    let (left, right) = fail.item.0.clone().unwrap();
    assert_ne!(left, right);
}

#[test]
fn filter_map_returns_none_when_predicate_fails() {
    let values = Generate::filter_map(0u8..=3, |value| (value % 2 == 0).then_some(value / 2))
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();

    assert_eq!(values, vec![Some(0), None, Some(1), None]);
}

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check(_, _)]
    fn filtered_u8_pair_preserves_predicate(left: u8, right: u8) {
        // Filter ensures left != right; verify the filter is respected.
        let result = (same(left), same(right))
            .filter(|(l, r)| l != r)
            .sample(1.0);
        match result {
            Some((l, r)) => assert_ne!(l, r),
            None => assert_eq!(left, right),
        }
    }

    #[check(0u8..=255)]
    fn filter_map_preserves_mapping(value: u8) {
        let result = same(value)
            .filter_map(|v| (v % 2 == 0).then_some(v / 2))
            .sample(1.0);
        if value % 2 == 0 {
            assert_eq!(result, Some(value / 2));
        } else {
            assert_eq!(result, None);
        }
    }
}
