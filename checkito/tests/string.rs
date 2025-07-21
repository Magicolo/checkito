pub mod common;
use common::*;
use std::collections::HashSet;

#[test]
fn checks_a_single_item() {
    assert_eq!("a constant".checks(|_| true).count(), 1);
}

#[test]
fn collects_exhaustively() {
    let set = Generate::collect::<String>('a'..='Z')
        .checks(Ok::<_, ()>)
        .map(|result| result.item())
        .collect::<HashSet<_>>();
    for letter in 'a'..='Z' {
        assert!(set.contains(letter.to_string().as_str()));
    }
}

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check(Generate::collect('a'..='z'), Generate::collect('A'..='Z'))]
    #[should_panic]
    fn fails_on_specific_input(left: String, right: String) {
        if left.len() + right.len() > 10 {
            assert_eq!(left.contains('z'), right.contains('Z'));
        }
    }
}
