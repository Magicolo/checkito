use checkito::*;

const COUNT: usize = 1024;

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
    assert_eq!(error.original, (false, false));
    assert_eq!(error.shrunk, None);
}