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
    char::generator()
        .bind(|value| value..value)
        .check(COUNT, |_| true)
        .unwrap();
}

#[test]
fn is_same() -> Result {
    char::generator()
        .bind(|value| (value, Same(value)))
        .check(COUNT, |&(left, right)| left == right)?;
    Ok(())
}

#[test]
fn is_ascii() -> Result {
    ascii().check(COUNT, |value| value.is_ascii())?;
    Ok(())
}

#[test]
fn is_digit() -> Result {
    digit().check(COUNT, |value| value.is_ascii_digit())?;
    Ok(())
}

#[test]
fn is_alphabetic() -> Result {
    letter().check(COUNT, |value| value.is_ascii_alphabetic())?;
    Ok(())
}

#[test]
fn full_does_not_panic() -> Result {
    char::generator().check(COUNT, |_| true)?;
    Ok(())
}

macro_rules! collection {
    ($m:ident, $t:ty $(, $i:ident)?) => {
        mod $m {
            use super::*;

            #[test]
            fn has_same_count() -> Result {
                (0..COUNT)
                    .bind(|count| (count, char::generator().collect_with::<_, $t>(count)))
                    .check(COUNT, |(count, value)| value $(.$i())? .count() == *count)?;
                Ok(())
            }

            #[test]
            fn is_ascii() -> Result {
                ascii().collect::<$t>().check(COUNT, |value| value $(.$i())? .all(|value| value.is_ascii()))?;
                Ok(())
            }

            #[test]
            fn is_digit() -> Result {
                digit().collect::<$t>().check(COUNT, |value| value $(.$i())? .all(|value| value.is_ascii_digit()))?;
                Ok(())
            }

            #[test]
            fn is_alphabetic() -> Result {
                letter().collect::<$t>().check(COUNT, |value| value $(.$i())? .all(|value| value.is_ascii_alphabetic()))?;
                Ok(())
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
