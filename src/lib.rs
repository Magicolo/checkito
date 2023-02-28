pub mod any;
pub mod array;
pub mod check;
pub mod collect;
pub mod constant;
pub mod filter;
pub mod filter_map;
pub mod flatten;
pub mod generate;
pub mod keep;
pub mod map;
pub mod primitive;
pub mod prove;
pub mod sample;
pub mod shrink;
pub mod size;
pub mod standard;
mod utility;

pub use crate::{
    generate::{FullGenerate, Generate, IntoGenerate},
    prove::Prove,
    shrink::Shrink,
};
use primitive::Range;
use size::Size;
use std::{
    fmt,
    ops::{self, Neg},
};

pub fn number<T>() -> impl Generate<Item = T>
where
    Size<Range<T>>: Generate<Item = T>,
    ops::RangeFull: TryInto<Size<Range<T>>>,
    <ops::RangeFull as TryInto<Size<Range<T>>>>::Error: fmt::Debug,
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
    ('a'..='z', 'A'..='Z').any().bind(|item| item.fuse())
}

pub fn digit() -> impl Generate<Item = char> {
    '0'..='9'
}

pub fn ascii() -> impl Generate<Item = char> {
    0 as char..127 as char
}

#[cfg(test)]
mod test;
