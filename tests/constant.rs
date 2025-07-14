#![cfg(feature = "constant")]

pub mod common;
use ::checkito::{
    primitive::{Constant, i32::I32},
    state::Range,
};
use checkito::primitive::usize::Usize;
use common::*;

#[test]
fn compiles_and_is_valid() {
    assert_eq!(constant!("a"), "a");
    assert_eq!(constant!(..), ..);

    assert_eq!(constant!(1), I32::<1>);
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

    assert_eq!(constant!(1usize), Usize::<1>);
    assert_eq!(constant!({ 1usize }), Usize::<1>);
    assert_eq!(constant!(1 as usize), Usize::<1>);
}
