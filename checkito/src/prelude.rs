use crate::{
    any::Any,
    array::Array,
    boxed::Boxed,
    cardinality::Cardinality,
    collect::{Collect, Count},
    convert::Convert,
    dampen::Dampen,
    filter::Filter,
    filter_map::FilterMap,
    flatten::Flatten,
    generate::Generate,
    keep::Keep,
    lazy::Lazy,
    map::Map,
    primitive::Number,
    same::Same,
    shrink::Shrinker,
    size::Size,
    state::Sizes,
    unify::Unify,
};
use core::marker::PhantomData;

/// Creates a generator that always produces the same value.
///
/// This is useful for creating parameterized unit tests or for fixing one
/// input to a test while letting others be generated randomly.
#[inline]
pub const fn same<T: Clone>(value: T) -> Same<T> {
    Same(value)
}

/// Creates a generator that randomly chooses one of a set of generators.
///
/// See [`any`](crate::any()).
#[inline]
pub const fn any<G: Generate>(generators: G) -> Any<G> {
    Any(generators)
}

/// Unifies a generator of a `orn::Or` type into a single type.
///
/// See [`unify`](crate::unify()).
#[inline]
pub const fn unify<G: Generate, T>(generator: G) -> Unify<G, T> {
    Unify(PhantomData, generator)
}

/// Creates a generator that yields the [`Generate::Shrink`] instances instead
/// of [`Generate::Item`].
#[inline]
pub const fn shrinker<G: Generate>(generator: G) -> Shrinker<G> {
    Shrinker(generator)
}

/// See [`Generate::map`].
#[inline]
pub const fn map<G: Generate, T, F: Fn(G::Item) -> T + Clone>(generator: G, map: F) -> Map<G, F> {
    Map(map, generator)
}

/// See [`Generate::flat_map`].
#[inline]
pub const fn flat_map<G: Generate, T: Generate, F: Fn(G::Item) -> T + Clone>(
    generator: G,
    map: F,
) -> Flatten<Map<G, F>> {
    flatten(self::map(generator, map))
}

/// See [`Generate::flatten`].
#[inline]
pub const fn flatten<G: Generate>(generator: G) -> Flatten<G>
where
    G::Item: Generate,
{
    Flatten(generator)
}

/// See [`Generate::filter`].
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

/// See [`Generate::filter_map`].
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

/// See [`Generate::boxed`].
#[rustversion::since(1.75)]
#[inline]
pub const fn boxed<G: Generate + 'static>(generator: Box<G>) -> Boxed<G::Item> {
    Boxed::new(generator)
}

/// See [`Generate::boxed`].
#[rustversion::before(1.75)]
#[inline]
pub fn boxed<G: Generate + 'static>(generator: Box<G>) -> Boxed<G::Item> {
    Boxed::new(generator)
}

/// See [`Generate::array`].
#[inline]
pub const fn array<G: Generate, const N: usize>(generator: G) -> Array<G, N> {
    Array(generator)
}

/// See [`Generate::collect`].
#[inline]
pub const fn collect<G: Generate, C: Count, F: FromIterator<G::Item>>(
    generator: G,
    count: C,
) -> Collect<G, C, F> {
    Collect {
        _marker: PhantomData,
        count,
        generator,
    }
}

/// See [`Generate::size`].
#[inline]
pub const fn size<G: Generate, S: Into<Sizes>, F: Fn(Sizes) -> S>(
    generator: G,
    map: F,
) -> Size<G, F> {
    Size(generator, map)
}

/// See [`Generate::dampen`].
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

/// See [`Generate::keep`].
#[inline]
pub const fn keep<G: Generate>(generator: G) -> Keep<G> {
    Keep(generator)
}

/// See [`Generate::convert`].
#[inline]
pub const fn convert<G: Generate, T: From<G::Item>>(generator: G) -> Convert<G, T> {
    Convert(PhantomData, generator)
}

/// Creates a generator from a regular expression at runtime.
///
/// If the regular expression parsing fails, an [`Err`] is returned. For
/// compile-time checked regexes, see the [`regex!`](crate::regex!) macro.
#[cfg(feature = "regex")]
#[inline]
pub fn regex(
    pattern: &str,
    repeats: Option<u32>,
) -> Result<crate::regex::Regex, crate::regex::Error> {
    crate::regex::Regex::new(pattern, repeats)
}

/// A generator for the full range of any [`Number`] type.
///
/// This is equivalent to `T::MIN..=T::MAX`.
#[inline]
pub const fn number<T: Number>() -> impl Generate<Item = T> {
    T::FULL
}

/// A generator for any non-negative [`Number`] type (includes `0`).
///
/// This is equivalent to `0..=T::MAX`.
#[inline]
pub const fn positive<T: Number>() -> impl Generate<Item = T> {
    T::POSITIVE
}

/// A generator for any non-positive [`Number`] type (includes `0`).
///
/// This is equivalent to `T::MIN..=0`.
#[inline]
pub const fn negative<T: Number>() -> impl Generate<Item = T> {
    T::NEGATIVE
}

/// A generator for ASCII letters (`a-z`, `A-Z`).
#[inline]
pub const fn letter() -> impl Generate<Item = char> {
    let generator = unify(any(('a'..='z', 'A'..='Z')));
    #[allow(clippy::let_and_return)]
    generator
}

/// A generator for ASCII digits (`0-9`).
#[inline]
pub const fn digit() -> impl Generate<Item = char> {
    let generator = '0'..='9';
    #[allow(clippy::let_and_return)]
    generator
}

/// A generator for all ASCII characters (0-127).
#[inline]
pub const fn ascii() -> impl Generate<Item = char> {
    let generator = 0 as char..=127 as char;
    #[allow(clippy::let_and_return)]
    generator
}

/// Creates a generator from a closure that produces a value.
///
/// This is useful for wrapping simple value creation in a generator.
///
/// # Examples
/// ```
/// # use checkito::*;
/// struct MyStruct(i32);
///
/// // A generator that always produces `MyStruct(42)`.
/// let generator = with(|| MyStruct(42));
/// ```
#[inline]
pub const fn with<T, F: Fn() -> T + Clone>(generator: F) -> impl Generate<Item = T> {
    let generator = map((), move |_| generator());
    #[allow(clippy::let_and_return)]
    generator
}

/// Defers the construction of a generator until it is used.
///
/// This is essential for creating recursive generators. See [`Lazy`] for
/// details.
#[inline]
pub const fn lazy<G: Generate, F: Fn() -> G>(generator: F) -> Lazy<G, F> {
    Lazy::new(generator)
}

/// Overrides both the static and dynamic cardinalities of a generator.
///
/// This is used when the context allows for a more precise cardinality than the
/// default estimate.
///
/// Providing an incorrect cardinality can cause unexpected behavior when
/// running [`crate::Check::check`].
#[inline]
pub const fn cardinality<G: Generate, const C: u128>(generator: G) -> Cardinality<G, C> {
    Cardinality(generator)
}
