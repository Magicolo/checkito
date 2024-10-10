use crate::{any::Any, primitive::number::Number, shrink, Generate, Same};

pub const fn same<T>(value: T) -> Same<T> {
    Same(value)
}

pub const fn any<G: Generate>(generators: G) -> Any<G> {
    Any(generators)
}

pub const fn shrinker<G: Generate>(generator: G) -> shrink::Shrinker<G> {
    shrink::Shrinker(generator)
}

/// From `MIN..=MAX`.
pub const fn number<T: Number>() -> impl Generate<Item = T> {
    T::FULL
}

/// From `0..=MAX`.
pub const fn positive<T: Number>() -> impl Generate<Item = T> {
    T::POSITIVE
}

/// From `MIN..=0`.
pub const fn negative<T: Number>() -> impl Generate<Item = T> {
    T::NEGATIVE
}

/// Ascii letters.
pub fn letter() -> impl Generate<Item = char> {
    ('a'..='z', 'A'..='Z').any().fuse::<char>()
}

/// Ascii digits.
pub fn digit() -> impl Generate<Item = char> {
    '0'..='9'
}

/// Ascii characters.
pub fn ascii() -> impl Generate<Item = char> {
    0 as char..127 as char
}

pub fn with<T, F: Fn() -> T + Clone>(generator: F) -> impl Generate<Item = T> {
    ().map(move |_| generator())
}

pub fn lazy<G: Generate, F: Fn() -> G + Clone>(generator: F) -> impl Generate<Item = G::Item> {
    ().flat_map(move |_| generator())
}
