pub mod common;
use common::*;

#[test]
fn finds_minimum() {
    let fail = <(usize, usize)>::generator()
        .check(|(left, right)| left >= right)
        .unwrap();
    assert_eq!(fail.item, (0, 1));
}

#[test]
fn integer_shrink_to_minimum() {
    for high in (1usize..1_000_000_000).samples(1_000) {
        let fail = usize::generator().check(|item| item < high).unwrap();
        assert_eq!(fail.item, high);
    }
}

#[test]
fn vec_removes_irrelevant_then_shrinks() {
    let fail = (..100usize)
        .collect::<Vec<_>>()
        .check(|items| items.len() < 10 || items.iter().all(|&item| item < 10))
        .unwrap();
    let shrunk = fail.item;
    assert_eq!(shrunk.len(), 10);
    assert_eq!(shrunk.iter().filter(|&&item| item == 10).count(), 1);
}

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check(1usize..=1_000_000_000)]
    fn integer_shrink_to_exact_boundary(high: usize) {
        let fail = usize::generator().check(|item| item < high).unwrap();
        assert_eq!(fail.item, high);
    }

    #[check(1u8..=u8::MAX)]
    fn u8_shrink_to_exact_boundary(high: u8) {
        let fail = u8::generator().check(|item| item < high).unwrap();
        assert_eq!(fail.item, high);
    }

    #[check(1i16..=i16::MAX)]
    fn i16_shrink_to_exact_positive_boundary(high: i16) {
        let fail = i16::generator().check(|item| item < high).unwrap();
        assert_eq!(fail.item, high);
    }
}
