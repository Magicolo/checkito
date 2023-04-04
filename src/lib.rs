pub mod any;
pub mod array;
pub mod check;
pub mod collect;
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
mod utility;

pub use crate::{
    generate::{FullGenerate, Generate, IntoGenerate},
    prove::Prove,
    shrink::Shrink,
    utility::Nudge,
};
use primitive::Range;
use std::{
    fmt,
    ops::{self, Neg},
};

/*
    TODO: Review `primitive::shrinked`.
    TODO: Find a way to separate `Generate` and `Shrink`.
    - Currently, `Generate::generate` must return a `Shrink` because some shrinkers need to store some of the generation state.
    - There may be a way to pass on that state using some mechanism in the `generate::State`?
    - This would add a lot of modularity to this library and be likely more performant.
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
    ('a'..='z', 'A'..='Z').any().bind(|item| item.fuse())
}

pub fn digit() -> impl Generate<Item = char> {
    '0'..='9'
}

pub fn ascii() -> impl Generate<Item = char> {
    0 as char..127 as char
}
