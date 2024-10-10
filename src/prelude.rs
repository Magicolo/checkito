use crate::{Generator, Same, any::Any, primitive::number::Number};

pub const fn same<T>(value: T) -> Same<T> {
    Same(value)
}

pub const fn any<G: Generator>(generators: G) -> Any<G> {
    Any(generators)
}

/// From `MIN..=MAX`.
pub const fn number<T: Number>() -> impl Generator<Item = T> {
    T::FULL
}

/// From `0..=MAX`.
pub const fn positive<T: Number>() -> impl Generator<Item = T> {
    T::POSITIVE
}

/// From `MIN..=0`.
pub const fn negative<T: Number>() -> impl Generator<Item = T> {
    T::NEGATIVE
}

/// Ascii letters.
pub fn letter() -> impl Generator<Item = char> {
    ('a'..='z', 'A'..='Z').any().fuse::<char>()
}

/// Ascii digits.
pub fn digit() -> impl Generator<Item = char> {
    '0'..='9'
}

/// Ascii characters.
pub fn ascii() -> impl Generator<Item = char> {
    0 as char..127 as char
}

pub fn with<T, F: Fn() -> T + Clone>(generator: F) -> impl Generator<Item = T> {
    ().map(move |_| generator())
}

pub fn lazy<G: Generator, F: Fn() -> G + Clone>(generator: F) -> impl Generator<Item = G::Item> {
    ().flat_map(move |_| generator())
}
