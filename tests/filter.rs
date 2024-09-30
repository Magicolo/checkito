pub mod common;
use common::*;

#[test]
fn filtered_pair_preserves_inequality() {
    assert!(<(String, String)>::generator()
        .filter(|(left, right)| left != right)
        .check(|pair| match pair {
            Some((left, right)) => left != right,
            None => true,
        })
        .is_none());
}

#[test]
fn filtered_array_preserves_inequality() {
    assert!(regex!("[a-z]+")
        .array::<3>()
        .filter(|[a, b, c]| a != b && b != c && a != c)
        .check(|array| match array {
            Some([a, b, c]) => a != b && b != c && a != c,
            None => true,
        })
        .is_none());
}

#[test]
fn shrinked_filter_preserves_inequality() {
    let fail = (
        <(String, String)>::generator().filter(|(left, right)| left != right),
        usize::generator(),
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
