pub mod common;
use checkito::same::Same;
use common::*;
use std::{
    collections::{LinkedList, VecDeque},
    rc::Rc,
    sync::Arc,
};

#[test]
#[should_panic]
fn empty_range() {
    assert!(
        char::full_gen()
            .flat_map(|value| value..value)
            .check(|_| true)
            .is_none()
    );
}

#[test]
fn is_same() {
    assert!(
        full::<char>()
            .flat_map(|value| (value, Same(value)))
            .check(|(left, right)| left == right)
            .is_none()
    );
}

#[test]
fn is_ascii() {
    assert!(ascii().check(|value| value.is_ascii()).is_none());
}

#[test]
fn is_digit() {
    assert!(digit().check(|value| value.is_ascii_digit()).is_none());
}

#[test]
fn is_alphabetic() {
    assert!(
        letter()
            .check(|value| value.is_ascii_alphabetic())
            .is_none()
    );
}

#[test]
fn full_does_not_panic() {
    assert!(char::full_gen().check(|_| true).is_none());
}

macro_rules! collection {
    ($m:ident, $t:ty, $i:ident) => {
        mod $m {
            use super::*;

            #[test]
            fn has_same_count() {
                assert!(
                    (0..100usize)
                        .into_gen()
                        .flat_map(|count| (count, char::full_gen().collect_with::<_, $t>(count)))
                        .check(|(count, value)| value.$i().count() == count)
                        .is_none()
                );
            }

            #[test]
            fn is_ascii() {
                assert!(
                    ascii()
                        .collect::<$t>()
                        .check(|value| value.$i().all(|value| value.is_ascii()))
                        .is_none()
                );
            }

            #[test]
            fn is_digit() {
                assert!(
                    digit()
                        .collect::<$t>()
                        .check(|value| value.$i().all(|value| value.is_ascii_digit()))
                        .is_none()
                );
            }

            #[test]
            fn is_alphabetic() {
                assert!(
                    letter()
                        .collect::<$t>()
                        .check(|value| value.$i().all(|value| value.is_ascii_alphabetic()))
                        .is_none()
                );
            }

            #[cfg(feature = "check")]
            #[allow(clippy::boxed_local)]
            mod check {
                use super::*;

                #[check(ascii().collect())]
                fn is_ascii(value: $t) {
                    assert!(value.$i().all(|value| value.is_ascii()));
                }

                #[check(digit().collect())]
                fn is_digit(value: $t) {
                    assert!(value.$i().all(|value| value.is_ascii_digit()));
                }

                #[check(letter().collect())]
                fn is_alphabetic(value: $t) {
                    assert!(value.$i().all(|value| value.is_ascii_alphabetic()));
                }
            }
        }
    };
}

collection!(string, String, chars);
collection!(vec_char, Vec<char>, iter);
collection!(vecdeque_char, VecDeque<char>, iter);
collection!(linked_list, LinkedList<char>, iter);
collection!(box_char, Box<[char]>, iter);
collection!(rc_char, Rc<[char]>, iter);
collection!(arc_char, Arc<[char]>, iter);

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check(char::full_gen().flat_map(|value| value..value))]
    #[should_panic]
    fn empty_range(_: char) {}

    #[check(char::full_gen().flat_map(|value| (value, Same(value))))]
    fn is_same(pair: (char, char)) {
        assert_eq!(pair.0, pair.1);
    }

    #[check(ascii())]
    fn is_ascii(value: char) {
        assert!(value.is_ascii());
    }

    #[check(digit())]
    fn is_digit(value: char) {
        assert!(value.is_ascii_digit());
    }

    #[check(letter())]
    fn is_alphabetic(value: char) {
        assert!(value.is_ascii_alphabetic());
    }

    #[check(_)]
    fn full_does_not_panic(_: char) {}
}
