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
    for high in (1usize..).samples(100) {
        if let Some(error) = usize::generator().check(|item| item < high) {
            assert!(error.item - high <= 1);
        }
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
