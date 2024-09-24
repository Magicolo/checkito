pub mod any;
pub mod array;
pub mod boxed;
pub mod check;
pub mod collect;
pub mod dampen;
pub mod filter;
pub mod filter_map;
pub mod flatten;
pub mod generate;
pub mod keep;
pub mod map;
pub mod primitive;
pub mod prove;
pub mod random;
pub mod regex;
pub mod same;
pub mod sample;
pub mod shrink;
pub mod size;
pub mod standard;
pub mod utility;

pub use crate::{
    generate::{FullGenerate, Generate, IntoGenerate},
    prove::Prove,
    shrink::{FullShrink, IntoShrink, Shrink},
};
pub use checkito_macro::{check, regex};
use core::{
    fmt,
    ops::{Neg, RangeFrom, RangeFull, RangeToInclusive},
};
use primitive::Range;

/*
    FIXME: specifying the seed in `[check(seed = 1)]` does not reproduce deterministic results.
    FIXME: skeptic test don't seem to be working...
    TODO: Provide named implementations for builtin generators.
    TODO: Review `primitive::shrinked`.
    TODO: Support for test macro with 'type expressions'?
        - Adds a lot of complexity for a limited syntactic convenience...
        - Support for 'or' generators would be nice; would require a fancy macro.


        - The '..' means that the rest of the fields should be filled with 'FullGenerate'.
        - Parameters that have no explicit generator will be filled with 'FullGenerate'.
        #[check(digit(), Person { name: letter().collect(), .. })]
        fn is_digit(value: char, person: Person, count: usize) {
            assert!(value.is_ascii_digit());
        }


        #[checkito::test(shrink = 1, count = COUNT, seed = 11376, errors = 1)]
        i: usize | u8 | i16, // Generates tests for every permutation of type expressions.
        r: 0..100usize | 256..1000, // The 'or' type expression allows to combine any other type expression.
        a: (Dog {} | Cat {}) as &dyn Animal, // Cast as a trait.
        p: "[a-zA-Z]+", // &str will be interpreted as regexes and checked at compile time. -> String
        s: String,
        s: char::collect_with::<String>(100), -> String
        d: digit(), -> char
        d: digit().collect::<String>(), -> String
        n: number<f64>(), // Use builtin generator functions. -> f64
        l: letter(), -> char
        l: letter().collect::<String>(), -> String
        a: [0..1238; 17], // If a number of elements is specified, this is an array.
        p: Person { name: "a-z", node: || Node::Null }, // Construct composite types inline.
        z: Dopple { name: p.name.clone() }, // Refer to previously defined values?
        v: [usize], // If no number of elements is specified, this is a vector.
        t: (usize, 0..10000),
        u: ()
    FIXME: Sometimes, integers don't shrink completely; they stop at 1 from the smallest value...
    - See `tests::shrink::integer_shrink_to_minimum`.
*/

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

pub fn negative<T: Neg + Default>() -> impl Generate<Item = T>
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
