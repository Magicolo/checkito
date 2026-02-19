#![cfg(feature = "constant")]

pub mod common;
use checkito::{
    array::Array,
    cardinality::Cardinality,
    collect::Collect,
    convert::Convert,
    dampen::Dampen,
    flatten::Flatten,
    keep::Keep,
    primitive::{
        Constant, Range, char::Char, i32::I32, i64::I64, i128::I128, isize::Isize, u128::U128,
        usize::Usize,
    },
};
use common::*;

#[test]
#[allow(clippy::unit_cmp)]
fn non_constant_remain_as_is() {
    assert_eq!(constant!("a"), "a");
    assert_eq!(constant!(..), ..);
    assert_eq!(constant!(()), ());
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
    assert_eq!(constant!((1i32,)), (I32::<1>,));
    assert_eq!(constant!({ (1,) }), (I32::<1>,));
    assert_eq!(constant!({ { 1 as u128 } }), U128::<1>);
    assert_eq!(constant!({ { { 1i128 } } }), I128::<1>);
    assert_eq!(constant!({ { { { 1i64 + 2 } } } }), I64::<3>);
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

#[test]
fn wrapping_generators_implement_constant() {
    // Keep
    let _keep = Keep::<I32<42>>::VALUE;

    // Dampen
    let _dampen = Dampen::<I32<100>>::VALUE;

    // Flatten
    let _flatten = Flatten::<I32<7>>::VALUE;

    // Array
    let _array = Array::<I32<5>, 3>::VALUE;

    // Convert
    let _convert = Convert::<I32<10>, i64>::VALUE;

    // Collect
    let _collect = Collect::<I32<1>, Range<Usize<0>, Usize<10>>, Vec<i32>>::VALUE;

    // Cardinality
    let _cardinality = Cardinality::<I32<99>, 42>::VALUE;
}
