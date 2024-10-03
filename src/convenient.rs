use crate::{Generate, IntoGenerate, primitive::Range};
use core::{
    fmt,
    ops::{RangeFrom, RangeFull, RangeToInclusive},
};

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
