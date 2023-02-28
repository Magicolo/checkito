use super::*;

#[test]
fn samples_true() {
    assert!(<bool>::generator().samples(COUNT).any(|value| value));
}

#[test]
fn samples_false() {
    assert!(<bool>::generator().samples(COUNT).any(|value| !value));
}
