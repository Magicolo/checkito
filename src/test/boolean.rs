use super::*;

#[test]
fn samples_true() {
    assert!(<bool>::generator().sample(COUNT).any(|value| value));
}

#[test]
fn samples_false() {
    assert!(<bool>::generator().sample(COUNT).any(|value| !value));
}
