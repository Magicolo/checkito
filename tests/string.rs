pub mod common;
use common::*;

#[test]
fn checks_a_single_item() {
    assert_eq!("a constant".into_gen().checks(|_| true).count(), 1);
}

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check(('a'..='z').into_gen().collect(), ('A'..='Z').into_gen().collect())]
    #[should_panic]
    fn fails_on_specific_input(left: String, right: String) {
        if left.len() + right.len() > 10 {
            assert_eq!(left.contains('z'), right.contains('Z'));
        }
    }
}
