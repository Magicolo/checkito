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
    let error = result.err().unwrap();
    assert_eq!(error.state.size(), 0.);
    assert_eq!(error.cause, Cause::Disprove(()));
    assert!(!error.item);
    assert!(!error.shrink);
    assert_eq!(error.shrinks, 0);
}
