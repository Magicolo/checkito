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
pub mod regex;
pub mod same;
pub mod sample;
pub mod shrink;
pub mod size;
pub mod standard;
pub mod utility;

pub use crate::{
    any::Unify,
    generate::{FullGenerate, Generate, IntoGenerate},
    prove::Prove,
    shrink::{FullShrink, IntoShrink, Shrink},
};
use primitive::Range;
use std::{
    fmt,
    ops::{self, Neg},
};

/*
    TODO: Review `primitive::shrinked`.
    FIXME: Sometimes, integers don't shrink completely; they stop at 1 from the smallest value...
    - See `tests::shrink::integer_shrink_to_minimum`.
*/

pub fn number<T>() -> impl Generate<Item = T>
where
    Range<T>: Generate<Item = T>,
    ops::RangeFull: TryInto<Range<T>>,
    <ops::RangeFull as TryInto<Range<T>>>::Error: fmt::Debug,
{
    (..).try_into().unwrap()
}

pub fn positive<T: Default>() -> impl Generate<Item = T>
where
    ops::RangeFrom<T>: IntoGenerate<Item = T>,
{
    (T::default()..).generator()
}

pub fn negative<T: Neg + Default>() -> impl Generate<Item = T>
where
    ops::RangeToInclusive<T>: IntoGenerate<Item = T>,
{
    (..=T::default()).generator()
}

pub fn letter() -> impl Generate<Item = char> {
    ('a'..='z', 'A'..='Z').generator().any().map(Unify::unify)
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
