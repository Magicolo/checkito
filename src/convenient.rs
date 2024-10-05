use crate::{
    Generator, IntoGenerator, Same,
    all::All,
    primitive::{Full, Range},
};
use core::{
    fmt,
    marker::PhantomData,
    ops::{RangeFrom, RangeFull, RangeToInclusive},
};

pub fn full<T>() -> Full<T>
where
    Full<T>: Generator<Item = T>,
{
    Full(PhantomData)
}

pub fn same<T>(value: T) -> Same<T>
where
    Same<T>: Generator<Item = T>,
{
    Same(value)
}

pub fn all<G>(generators: G) -> All<G>
where
    All<G>: Generator,
{
    All(generators)
}

pub fn number<T>() -> Range<T>
where
    Range<T>: Generator<Item = T>,
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
    ('a'..='z', 'A'..='Z').into_gen().any().map(|or| or.into())
}

pub fn digit() -> impl Generator<Item = char> {
    ('0'..='9').into_gen()
}

pub fn ascii() -> impl Generator<Item = char> {
    (0 as char..127 as char).into_gen()
}

pub fn with<T, F: Fn() -> T + Clone>(generate: F) -> impl Generator<Item = T> {
    ().into_gen().map(move |_| generate())
}

pub fn lazy<G: Generator, F: Fn() -> G + Clone>(generate: F) -> impl Generator<Item = G::Item> {
    ().into_gen().flat_map(move |_| generate())
}
