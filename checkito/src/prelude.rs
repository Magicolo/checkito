//! A prelude of commonly used items, brought into scope automatically
//! by `use checkito::*`.
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
/// See [`any`](crate::any()) for more details.
#[inline]
pub const fn any<G: Generate>(generators: G) -> Any<G> {
    Any(generators)
}

/// Unifies a generator of a "choice" type into a single type.
///
/// See [`unify`](crate::unify()) for more details.
#[inline]
pub const fn unify<G: Generate, T>(generator: G) -> Unify<G, T> {
    Unify(PhantomData, generator)
}

/// Creates a generator that yields the [`crate::shrink::Shrink`] instances of another generator.
#[inline]
pub const fn shrinker<G: Generate>(generator: G) -> Shrinker<G> {
    Shrinker(generator)
}

/// Creates a new generator that transforms the output of another.
///
/// See [`Map`] for more details.
#[inline]
pub const fn map<G: Generate, T, F: Fn(G::Item) -> T + Clone>(generator: G, map: F) -> Map<G, F> {
    Map(map, generator)
}

/// Creates a new generator by applying a function to the output of another,
/// and then flattening the result.
///
/// See [`Generate::flat_map`] for more details.
#[inline]
pub const fn flat_map<G: Generate, T: Generate, F: Fn(G::Item) -> T + Clone>(
    generator: G,
    map: F,
) -> Flatten<Map<G, F>> {
    flatten(self::map(generator, map))
}

/// Flattens a generator of generators.
///
/// See [`Flatten`] for more details.
#[inline]
pub const fn flatten<G: Generate>(generator: G) -> Flatten<G>
where
    G::Item: Generate,
{
    Flatten(generator)
}

/// Creates a new generator that discards values that don't match a predicate.
///
/// See [`Filter`] for more details.
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

/// Creates a new generator that both filters and maps values simultaneously.
///
/// See [`FilterMap`] for more details.
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

/// Wraps a generator in a [`Boxed`] to erase its concrete type.
///
/// See [`Boxed`] for more details.
#[rustversion::since(1.75)]
#[inline]
pub const fn boxed<G: Generate + 'static>(generator: Box<G>) -> Boxed<G::Item> {
    Boxed::new(generator)
}

/// Wraps a generator in a [`Boxed`] to erase its concrete type.
///
/// See [`Boxed`] for more details.
#[rustversion::before(1.75)]
#[inline]
pub fn boxed<G: Generate + 'static>(generator: Box<G>) -> Boxed<G::Item> {
    Boxed::new(generator)
}

/// Creates a generator that produces a fixed-size array.
///
/// See [`Array`] for more details.
#[inline]
pub const fn array<G: Generate, const N: usize>(generator: G) -> Array<G, N> {
    Array(generator)
}

/// Creates a generator that produces a collection of items.
///
/// See [`Collect`] for more details.
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

/// Creates a generator that modifies the `size` parameter for subsequent generation.
///
/// See [`Size`] for more details.
#[inline]
pub const fn size<G: Generate, S: Into<Sizes>, F: Fn(Sizes) -> S>(
    generator: G,
    map: F,
) -> Size<G, F> {
    Size(generator, map)
}

/// Dampens the `size` of generation, typically for recursive structures.
///
/// See [`Dampen`] for more details.
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

/// Creates a generator whose values are not shrunk.
///
/// See [`Keep`] for more details.
#[inline]
pub const fn keep<G: Generate>(generator: G) -> Keep<G> {
    Keep(generator)
}

/// Creates a new generator that converts the output of another using [`From`].
///
/// See [`Convert`] for more details.
#[inline]
pub const fn convert<G: Generate, T: From<G::Item>>(generator: G) -> Convert<G, T> {
    Convert(PhantomData, generator)
}

#[cfg(feature = "regex")]
use crate::regex::{Error, Regex};
/// Creates a generator from a regular expression at runtime.
///
/// This function will parse the regex pattern and return a [`Result`]. If parsing
/// fails, an [`Error`] is returned. For compile-time checked regexes, see the
/// [`regex!`](crate::regex!) macro.
#[cfg(feature = "regex")]
#[inline]
pub fn regex(pattern: &str, repeats: Option<u32>) -> Result<Regex, Error> {
    Regex::new(pattern, repeats)
}

/// A generator for the full range of any [`Number`] type.
///
/// This is equivalent to `T::MIN..=T::MAX`.
#[inline]
pub const fn number<T: Number>() -> impl Generate<Item = T> {
    T::FULL
}

/// A generator for any non-negative [`Number`] type.
///
/// This is equivalent to `0..=T::MAX`.
#[inline]
pub const fn positive<T: Number>() -> impl Generate<Item = T> {
    T::POSITIVE
}

/// A generator for any non-positive [`Number`] type.
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
/// This is essential for creating recursive generators. See [`Lazy`] for details.
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
