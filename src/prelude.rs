use crate::{
    any::Any, array::Array, boxed::Boxed, check::Sizes, collect::Collect, convert::Convert,
    dampen::Dampen, filter::Filter, filter_map::FilterMap, flatten::Flatten, generate::Generate,
    keep::Keep, map::Map, primitive::number::Number, same::Same, shrink::Shrinker, size::Size,
    unify::Unify,
};
use core::marker::PhantomData;

#[inline]
pub const fn same<T: Clone>(value: T) -> Same<T> {
    Same(value)
}

#[inline]
pub const fn any<G: Generate>(generators: G) -> Any<G> {
    Any(generators)
}

#[inline]
pub const fn unify<G: Generate, T>(generator: G) -> Unify<G, T> {
    Unify(PhantomData, generator)
}

#[inline]
pub const fn shrinker<G: Generate>(generator: G) -> Shrinker<G> {
    Shrinker(generator)
}

#[inline]
pub const fn map<G: Generate, T, F: Fn(G::Item) -> T + Clone>(generator: G, map: F) -> Map<G, F> {
    Map(map, generator)
}

#[inline]
pub const fn flat_map<G: Generate, T: Generate, F: Fn(G::Item) -> T + Clone>(
    generator: G,
    map: F,
) -> Flatten<Map<G, F>> {
    flatten(self::map(generator, map))
}

#[inline]
pub const fn flatten<G: Generate>(generator: G) -> Flatten<G>
where
    G::Item: Generate,
{
    Flatten(generator)
}

#[inline]
pub const fn filter<G: Generate, F: Fn(&G::Item) -> bool + Clone>(
    generator: G,
    filter: F,
    retries: usize,
) -> Filter<G, F> {
    Filter {
        generator,
        filter,
        retries,
    }
}

#[inline]
pub const fn filter_map<G: Generate, T, F: Fn(G::Item) -> Option<T> + Clone>(
    generator: G,
    filter: F,
    retries: usize,
) -> FilterMap<G, F> {
    FilterMap {
        generator,
        filter,
        retries,
    }
}

#[inline]
pub fn boxed<G: Generate + 'static>(generator: Box<G>) -> Boxed<G::Item> {
    Boxed::new(generator)
}

#[inline]
pub const fn array<G: Generate, const N: usize>(generator: G) -> Array<G, N> {
    Array(generator)
}

#[inline]
pub const fn collect<G: Generate, C: Generate<Item = usize>, F: FromIterator<G::Item>>(
    generator: G,
    count: C,
    minimum: Option<usize>,
) -> Collect<G, C, F> {
    Collect {
        _marker: PhantomData,
        count,
        minimum,
        generator,
    }
}

#[inline]
pub const fn size<G: Generate, S: Into<Sizes>, F: Fn(Sizes) -> S>(
    generator: G,
    map: F,
) -> Size<G, F> {
    Size(generator, map)
}

#[inline]
pub const fn dampen<G: Generate>(
    generator: G,
    pressure: f64,
    deepest: usize,
    limit: usize,
) -> Dampen<G> {
    Dampen {
        pressure,
        deepest,
        limit,
        generator,
    }
}

#[inline]
pub const fn keep<G: Generate>(generator: G) -> Keep<G> {
    Keep(generator)
}

#[inline]
pub const fn convert<G: Generate, T: From<G::Item>>(generator: G) -> Convert<G, T> {
    Convert(PhantomData, generator)
}

#[cfg(feature = "regex")]
use crate::regex::{Error, Regex};
#[cfg(feature = "regex")]
#[inline]
pub fn regex(pattern: &str, repeats: Option<u32>) -> Result<Regex, Error> {
    Regex::new(pattern, repeats)
}

/// From `MIN..=MAX`.
#[inline]
pub const fn number<T: Number>() -> impl Generate<Item = T> {
    T::FULL
}

/// From `0..=MAX`.
#[inline]
pub const fn positive<T: Number>() -> impl Generate<Item = T> {
    T::POSITIVE
}

/// From `MIN..=0`.
#[inline]
pub const fn negative<T: Number>() -> impl Generate<Item = T> {
    T::NEGATIVE
}

/// Ascii letters.
#[inline]
pub const fn letter() -> impl Generate<Item = char> {
    let generator = unify(any(('a'..='z', 'A'..='Z')));
    #[allow(clippy::let_and_return)]
    generator
}

/// Ascii digits.
#[inline]
pub const fn digit() -> impl Generate<Item = char> {
    let generator = '0'..='9';
    #[allow(clippy::let_and_return)]
    generator
}

/// Ascii characters.
#[inline]
pub const fn ascii() -> impl Generate<Item = char> {
    let generator = 0 as char..=127 as char;
    #[allow(clippy::let_and_return)]
    generator
}

#[inline]
pub const fn with<T, F: Fn() -> T + Clone>(generator: F) -> impl Generate<Item = T> {
    let generator = map((), move |_| generator());
    #[allow(clippy::let_and_return)]
    generator
}

#[inline]
pub const fn lazy<G: Generate, F: Fn() -> G + Clone>(
    generator: F,
) -> impl Generate<Item = G::Item> {
    let generator = flat_map((), move |_| generator());
    #[allow(clippy::let_and_return)]
    generator
}
