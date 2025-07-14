#![cfg(feature = "constant")]

pub mod common;
use ::checkito::{
    primitive::{Constant, i32::I32},
    state::Range,
};
use checkito::primitive::{char::Char, isize::Isize, usize::Usize};
use common::*;

#[test]
fn non_constant_remain_as_is() {
    assert_eq!(constant!("a"), "a");
    assert_eq!(constant!(..), ..);
}

#[test]
fn constant_value_is_converted() {
    assert_eq!(constant!(1), I32::<1>);
    assert_eq!(constant!(1usize), Usize::<1>);
    assert_eq!(constant!(-1isize), Isize::<{ -1 }>);
}

#[test]
fn constant_expression_is_converted() {
    assert_eq!(constant!({ 1usize }), Usize::<1>);
    assert_eq!(constant!(1 as usize), Usize::<1>);
    assert_eq!(constant!(1usize + 2), Usize::<3>);
    assert_eq!(constant!(1usize + 2usize), Usize::<3>);
    assert_eq!(constant!(1 + 2usize), Usize::<3>);
}

#[test]
fn constant_range_is_converted() {
    assert_eq!(
        constant!(2..),
        Range::<I32::<2>, I32::<{ i32::MAX }>>::VALUE
    );
    assert_eq!(constant!(3..100), Range::<I32::<3>, I32::<99>>::VALUE);
    assert_eq!(constant!(4..=1000), Range::<I32::<4>, I32::<1000>>::VALUE);
    assert_eq!(
        constant!(..2000),
        Range::<I32::<{ i32::MIN }>, I32::<1999>>::VALUE
    );
    assert_eq!(
        constant!(..=3000),
        Range::<I32::<{ i32::MIN }>, I32::<3000>>::VALUE
    );

    assert_eq!(
        constant!(2isize..),
        Range::<Isize::<2>, Isize::<{ isize::MAX }>>::VALUE
    );
    assert_eq!(
        constant!(3isize..100),
        Range::<Isize::<3>, Isize::<99>>::VALUE
    );
    assert_eq!(
        constant!(4isize..=1000isize),
        Range::<Isize::<4>, Isize::<1000>>::VALUE
    );
    assert_eq!(
        constant!(..2000isize),
        Range::<Isize::<{ isize::MIN }>, Isize::<1999>>::VALUE
    );
    assert_eq!(
        constant!(..=3000isize),
        Range::<Isize::<{ isize::MIN }>, Isize::<3000>>::VALUE
    );

    assert_eq!(
        constant!('a'..),
        Range::<Char::<'a'>, Char::<{ char::MAX }>>::VALUE
    );
    assert_eq!(
        constant!('b'..'z'),
        Range::<Char::<'b'>, Char::<'y'>>::VALUE
    );
    assert_eq!(
        constant!('c'..='Z'),
        Range::<Char::<'c'>, Char::<'Z'>>::VALUE
    );
    assert_eq!(
        constant!(..'Y'),
        Range::<Char::<{ char::MIN }>, Char::<'X'>>::VALUE
    );
    assert_eq!(
        constant!(..='0'),
        Range::<Char::<{ char::MIN }>, Char::<'0'>>::VALUE
    );
}
