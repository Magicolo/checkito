pub mod common;
use common::*;

#[test]
fn finds_minimum() {
    let result = <(usize, usize)>::generator().check(COUNT, |&(left, right)| left >= right);
    let error = result.err().unwrap();
    assert_eq!(*error.shrunk(), (0, 1));
}

#[test]
fn integer_shrink_to_minimum() {
    for high in (1usize..).samples(COUNT) {
        if let Err(error) = usize::generator().check(COUNT, |item| *item < high) {
            assert!(*error.shrunk() - high <= 1);
        }
    }
}

#[test]
fn vec_removes_irrelevant_then_shrinks() {
    let error = (..100usize)
        .collect::<Vec<_>>()
        .check(COUNT, |items| {
            items.len() < 10 || items.iter().all(|&item| item < 10)
        })
        .err()
        .unwrap();
    let shrunk = error.shrunk();
    assert_eq!(shrunk.len(), 10);
    assert_eq!(shrunk.iter().filter(|&&item| item == 10).count(), 1);
}
