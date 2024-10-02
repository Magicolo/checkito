#![forbid(unsafe_code)]

pub mod any;
pub mod array;
pub mod boxed;
pub mod check;
pub mod collect;
pub mod dampen;
pub mod filter;
pub mod filter_map;
pub mod flatten;
pub mod generate;
pub mod keep;
pub mod map;
pub mod primitive;
pub mod prove;
pub mod random;
pub mod regex;
pub mod same;
pub mod sample;
pub mod shrink;
pub mod size;
pub mod standard;
pub mod utility;

pub use crate::{
    check::Check,
    generate::{FullGenerate, Generate, IntoGenerate},
    prove::Prove,
    same::Same,
    sample::Sample,
    shrink::{FullShrink, IntoShrink, Shrink},
};
pub use checkito_macro::{check, regex};
use core::{
    fmt,
    ops::{RangeFrom, RangeFull, RangeToInclusive},
};
use primitive::Range;

pub fn number<T>() -> impl Generate<Item = T>
where
    Range<T>: Generate<Item = T>,
    RangeFull: TryInto<Range<T>>,
    <RangeFull as TryInto<Range<T>>>::Error: fmt::Debug,
{
    (..).try_into().unwrap()
}

pub fn positive<T: Default>() -> impl Generate<Item = T>
where
    RangeFrom<T>: IntoGenerate<Item = T>,
{
    (T::default()..).generator()
}

pub fn negative<T: Default>() -> impl Generate<Item = T>
where
    RangeToInclusive<T>: IntoGenerate<Item = T>,
{
    (..=T::default()).generator()
}

pub fn letter() -> impl Generate<Item = char> {
    ('a'..='z', 'A'..='Z').generator().any().map(|or| or.into())
}

pub fn digit() -> impl Generate<Item = char> {
    ('0'..='9').generator()
}

pub fn ascii() -> impl Generate<Item = char> {
    (0 as char..127 as char).generator()
}

pub fn with<T, F: Fn() -> T + Clone>(generate: F) -> impl Generate<Item = T> {
    ().map(move |_| generate())
}

pub fn lazy<G: Generate, F: Fn() -> G + Clone>(generate: F) -> impl Generate<Item = G::Item> {
    ().flat_map(move |_| generate())
}
