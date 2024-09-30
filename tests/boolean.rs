pub mod common;
use common::*;

#[test]
fn samples_true() {
    assert!(<bool>::generator().samples(100).any(|value| value));
}

#[test]
fn samples_false() {
    assert!(<bool>::generator().samples(100).any(|value| !value));
}

#[test]
fn first_size_is_0_and_false() {
    let result = bool::generator().check(|_| false);
    let fail = result.unwrap();
    assert_eq!(fail.state.size(), 0.);
    assert_eq!(fail.cause, Cause::Disprove(()));
    assert!(!fail.item);
    assert_eq!(fail.shrinks, 0);
}
