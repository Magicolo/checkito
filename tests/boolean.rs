pub mod common;
use common::*;

#[test]
fn samples_true() {
    assert!(<bool>::generator().samples(COUNT).any(|value| value));
}

#[test]
fn samples_false() {
    assert!(<bool>::generator().samples(COUNT).any(|value| !value));
}

#[test]
fn first_size_is_0_and_false() {
    let result = bool::generator().check(COUNT, |_| false);
    let error = result.err().unwrap();
    assert_eq!(error.state.size(), 0.);
    assert_eq!(error.cause, Cause::Disprove(false));
    assert!(!error.original);
    assert_eq!(error.shrunk, None);
}
