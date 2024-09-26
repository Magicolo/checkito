pub mod common;
use common::*;

#[check(Generate::collect('a'..='z'), Generate::collect('A'..='Z'))]
#[should_panic]
fn fails_on_specific_input(left: String, right: String) {
    if left.len() + right.len() > 10 {
        assert_eq!(left.contains('z'), right.contains('Z'));
    }
}
