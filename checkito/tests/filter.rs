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
fn filter_map_with_zero_retries_can_return_none() {
    let values = (0u8..=3)
        .filter_map_with(0, |value| (value % 2 == 0).then_some(value / 2))
        .checks(Ok::<_, ()>)
        .map(|result| result.into_item())
        .collect::<Vec<_>>();

    assert_eq!(values, vec![Some(0), None, Some(1), None]);
}

#[test]
fn filter_map_with_retries_calls_mapping_for_each_attempt() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let calls = Arc::new(AtomicUsize::new(0));
    let values = {
        let calls = Arc::clone(&calls);
        same(1u8)
            .filter_map_with(4, move |value| {
                calls.fetch_add(1, Ordering::SeqCst);
                (value == 2).then_some(value)
            })
            .samples(3)
            .collect::<Vec<_>>()
    };

    assert_eq!(values, vec![None, None, None]);
    assert_eq!(calls.load(Ordering::SeqCst), 15);
}
