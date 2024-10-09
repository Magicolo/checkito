use crate::{Generator, IntoGenerator, Same, all::All, any::Any, primitive::Range};
use core::{
    fmt,
    ops::{RangeFrom, RangeFull, RangeToInclusive},
};

pub const fn same<T>(value: T) -> Same<T> {
    Same(value)
}

pub fn all<G: IntoGenerator>(generators: G) -> All<G::IntoGen> {
    All(generators.into_gen())
}

pub fn any<G: IntoGenerator>(generators: G) -> Any<G::IntoGen> {
    Any(generators.into_gen())
}

pub fn number<T>() -> Range<T>
where
    Range<T>: Generator,
    RangeFull: TryInto<Range<T>>,
    <RangeFull as TryInto<Range<T>>>::Error: fmt::Debug,
{
    (..).try_into().unwrap()
}

pub fn positive<T: Default>() -> impl Generator<Item = T>
where
    RangeFrom<T>: IntoGenerator<Item = T>,
{
    (T::default()..).into_gen()
}

pub fn negative<T: Default>() -> impl Generator<Item = T>
where
    RangeToInclusive<T>: IntoGenerator<Item = T>,
{
    (..=T::default()).into_gen()
}

pub fn letter() -> impl Generator<Item = char> {
    ('a'..='z', 'A'..='Z').into_gen().any().fuse::<char>()
}

pub fn digit() -> impl Generator<Item = char> {
    ('0'..='9').into_gen()
}

pub fn ascii() -> impl Generator<Item = char> {
    (0 as char..127 as char).into_gen()
}

pub fn with<T, F: Fn() -> T + Clone>(generator: F) -> impl Generator<Item = T> {
    ().into_gen().map(move |_| generator())
}

pub fn lazy<G: Generator, F: Fn() -> G + Clone>(generator: F) -> impl Generator<Item = G::Item> {
    ().into_gen().flat_map(move |_| generator())
}
