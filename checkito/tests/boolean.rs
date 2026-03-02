pub mod common;
use common::*;

#[test]
fn samples_true() {
    assert!(bool::generator().samples(100).any(|value| value));
}

#[test]
fn samples_false() {
    assert!(bool::generator().samples(100).any(|value| !value));
}

#[test]
fn first_size_is_0_and_false() {
    let result = bool::generator().check(|_| false);
    let fail = result.unwrap();
    assert_eq!(fail.state.size(), 0.);
    assert_eq!(fail.cause, Cause::Disprove(()));
    assert!(!fail.item);
    assert!(fail.shrinks <= 1);
}

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check(10usize..=500)]
    fn samples_contain_true_for_arbitrary_count(count: usize) {
        assert!(bool::generator().samples(count).any(|value| value));
    }

    #[check(10usize..=500)]
    fn samples_contain_false_for_arbitrary_count(count: usize) {
        assert!(bool::generator().samples(count).any(|value| !value));
    }
}
