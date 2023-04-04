use checkito::{same::Same, *};
use std::{
    collections::{LinkedList, VecDeque},
    rc::Rc,
    sync::Arc,
};

type Result<T> = std::result::Result<(), check::Error<T, bool>>;
const COUNT: usize = 1024;

#[test]
#[should_panic]
fn empty_range() {
    char::generator()
        .bind(|value| value..value)
        .check(COUNT, |_| true)
        .unwrap();
}

#[test]
fn is_same() -> Result<(char, char)> {
    char::generator()
        .bind(|value| (value, Same(value)))
        .check(COUNT, |&(left, right)| left == right)
}

#[test]
fn is_ascii() -> Result<char> {
    ascii().check(COUNT, |value| value.is_ascii())
}

#[test]
fn is_digit() -> Result<char> {
    digit().check(COUNT, |value| value.is_ascii_digit())
}

#[test]
fn is_alphabetic() -> Result<char> {
    letter().check(COUNT, |value| value.is_ascii_alphabetic())
}

#[test]
fn full_does_not_panic() -> Result<char> {
    char::generator().check(COUNT, |_| true)
}

macro_rules! collection {
    ($m:ident, $t:ty $(, $i:ident)?) => {
        mod $m {
            use super::*;

            #[test]
            fn has_same_count() -> Result<(usize, $t)> {
                (0..COUNT)
                    .bind(|count| (count, char::generator().collect_with::<_, $t>(count)))
                    .check(COUNT, |(count, value)| value $(.$i())? .count() == *count)
            }

            #[test]
            fn is_ascii() -> Result<$t> {
                ascii().collect::<$t>().check(COUNT, |value| value $(.$i())? .all(|value| value.is_ascii()))
            }

            #[test]
            fn is_digit() -> Result<$t> {
                digit().collect::<$t>().check(COUNT, |value| value $(.$i())? .all(|value| value.is_ascii_digit()))
            }

            #[test]
            fn is_alphabetic() -> Result<$t> {
                letter().collect::<$t>().check(COUNT, |value| value $(.$i())? .all(|value| value.is_ascii_alphabetic()))
            }
        }
    };
}

collection!(string, String, chars);
collection!(vec_char, Vec<char>, into_iter);
collection!(vecdeque_char, VecDeque<char>, into_iter);
collection!(linked_list, LinkedList<char>, into_iter);
collection!(box_char, Box<[char]>, into_iter);
collection!(rc_char, Rc<[char]>, into_iter);
collection!(arc_char, Arc<[char]>, into_iter);
