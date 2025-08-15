use crate::{
    any::Any,
    array::Array,
    boxed::Boxed,
    collect::{Collect, Count},
    convert::Convert,
    dampen::Dampen,
    filter::Filter,
    filter_map::FilterMap,
    flatten::Flatten,
    keep::Keep,
    map::Map,
    prelude,
    primitive::{Constant, Range, usize::Usize},
    shrink::Shrink,
    size::Size,
    state::{Sizes, State},
    unify::Unify,
    COLLECTS, RETRIES,
};
use core::iter::FromIterator;

/// Provides a default, parameterless generator for a type.
///
/// This trait should be implemented for any type that has a single, canonical
/// way of being generated. It is used by the `_` and `..` placeholders in the
/// [`macro@crate::check`] macro to infer a generator automatically.
///
/// For example, `u8` implements `FullGenerate`, and its `generator()` returns a
/// generator for the full range of `u8` (`0..=255`). `String` also implements
/// it, returning a generator for arbitrary strings.
///
/// # Examples
///
/// ```
/// # use checkito::{FullGenerate, Generate, check};
/// struct MyType(u8);
///
/// impl FullGenerate for MyType {
///     type Item = Self;
///     type Generator = impl Generate<Item = Self>;
///     fn generator() -> Self::Generator {
///         // The generator for `MyType` will produce values with `u8` from 0 to 10.
///         (0..=10).map(MyType)
///     }
/// }
///
/// #[check(_)] // The `_` infers `MyType::generator()`
/// fn my_type_is_small(value: MyType) {
///     assert!(value.0 <= 10);
/// }
/// ```
pub trait FullGenerate {
    /// The type of the value that the generator produces.
    type Item;
    /// The concrete [`Generate`] type returned by `generator()`.
    type Generator: Generate<Item = Self::Item>;
    /// Creates a default generator for the type.
    fn generator() -> Self::Generator;
}

/// The core trait for all value generators.
///
/// `Generate` is a highly composable trait, much like [`Iterator`]. It provides a
/// rich set of methods (often called "combinators") that can be chained to build
/// complex generators from simpler ones.
///
/// A generator is a type that knows how to produce a random value and a corresponding
/// [`Shrink`] instance, which can simplify that value if it causes a test to fail.
#[must_use = "generators do nothing until used"]
pub trait Generate {
    /// The type of the value that this generator produces.
    type Item;
    /// The [`Shrink`] implementation that corresponds to this generator.
    type Shrink: Shrink<Item = Self::Item>;

    /// The static cardinality of the generated values.
    ///
    /// This value represents the number of unique items a generator's *type* can
    /// produce across all its possible configurations. For example, `bool` can
    /// produce 2 unique values, so its `CARDINALITY` is `Some(2)`. For generators
    /// with an effectively infinite or incalculable number of values (like `String`),
    /// this is `None`.
    ///
    /// This is used by the [`crate::Check`] engine to determine if a test can be run
    /// exhaustively.
    const CARDINALITY: Option<u128>;

    /// Generates a random value and its associated [`Shrink`] instance.
    ///
    /// This is the primary method of the trait. It takes the current generation
    /// [`State`], which contains the random number source and other parameters,
    /// and produces a [`Shrink`] instance. This shrinker can then be used to get
    /// the generated value via [`Shrink::item`] and to shrink it via [`Shrink::shrink`].
    fn generate(&self, state: &mut State) -> Self::Shrink;

    /// Returns the dynamic cardinality of the generated values.
    ///
    /// This value represents the number of unique items a *specific instance* of a
    /// generator can produce. For example, for the range `0..10`, the cardinality
    /// is `Some(10)`.
    ///
    /// If not overridden, this defaults to the static `CARDINALITY`.
    fn cardinality(&self) -> Option<u128> {
        Self::CARDINALITY
    }

    /// Wraps `self` in a boxed [`Generate`] to erase its concrete type.
    ///
    /// This is essential when dealing with recursive generators or when you need to
    /// return different generator types from the same function.
    ///
    /// # Examples
    ///
    /// Creating a generator for a recursive tree structure:
    /// ```
    /// # use checkito::*;
    /// enum Node {
    ///     Leaf,
    ///     Branch(Vec<Node>),
    /// }
    ///
    /// fn node() -> impl Generate<Item = Node> {
    ///     (
    ///         with(|| Node::Leaf),
    ///         // Without `lazy`, the call to `node()` would recurse infinitely.
    ///         // Without `dampen`, the tree could grow exponentially large.
    ///         // Without `boxed`, the recursive type would be infinitely large.
    ///         lazy(node).collect().map(Node::Branch).dampen().boxed(),
    ///     )
    ///         .any()
    ///         .unify()
    /// }
    /// ```
    ///
    /// Returning different generators from a function:
    /// ```
    /// # use checkito::*;
    /// fn choose(choose: bool) -> impl Generate<Item = char> {
    ///     if choose {
    ///         // The two branches have different concrete types, so they must be
    ///         // boxed to have the same return type.
    ///         letter().boxed()
    ///     } else {
    ///         digit().boxed()
    ///     }
    /// }
    /// ```
    fn boxed(self) -> Boxed<Self::Item>
    where
        Self: Sized + 'static,
    {
        prelude::boxed(Box::new(self))
    }

    /// Creates a new generator that transforms the output of `self`.
    ///
    /// This is one of the most common combinators, analogous to [`Iterator::map`].
    ///
    /// # Examples
    /// ```
    /// # use checkito::*;
    /// let evens = (0..10).map(|x| x * 2);
    ///
    /// evens.check(|x| assert_eq!(x % 2, 0));
    /// ```
    fn map<T, F: Fn(Self::Item) -> T + Clone>(self, map: F) -> Map<Self, F>
    where
        Self: Sized,
    {
        prelude::map(self, map)
    }

    /// Creates a new generator that discards values that don't match a predicate.
    ///
    /// This is analogous to [`Iterator::filter`]. Because the filter might always
    /// fail, this generator produces an [`Option<Self::Item>`], where `None` indicates
    /// that a matching value could not be found within a limited number of retries.
    ///
    /// This is a shorthand for [`Generate::filter_with`] with a default number of retries.
    ///
    /// # Examples
    /// ```
    /// # use checkito::*;
    /// // A generator for even numbers between 0 and 100.
    /// let evens = (0..100).filter(|&x| x % 2 == 0);
    ///
    /// // The generated value is an `Option`.
    /// evens.check(|x: Option<i32>| assert!(x.unwrap() % 2 == 0));
    /// ```
    fn filter<F: Fn(&Self::Item) -> bool + Clone>(self, filter: F) -> Filter<Self, F>
    where
        Self: Sized,
    {
        prelude::filter(self, filter, RETRIES)
    }

    /// Creates a new generator that discards values that don't match a predicate,
    /// with a configurable number of retries.
    ///
    /// See [`Generate::filter`] for more details.
    fn filter_with<F: Fn(&Self::Item) -> bool + Clone>(
        self,
        retries: usize,
        filter: F,
    ) -> Filter<Self, F>
    where
        Self: Sized,
    {
        prelude::filter(self, filter, retries)
    }

    /// Creates a new generator that both filters and maps values simultaneously.
    ///
    /// This is analogous to [`Iterator::filter_map`]. The provided function returns
    /// an [`Option`], where `Some(value)` is kept and `None` is discarded. This is
    /// more efficient than chaining [`Generate::filter`] and [`Generate::map`].
    ///
    /// This is a shorthand for [`Generate::filter_map_with`] with a default number
    /// of retries.
    ///
    /// # Examples
    /// ```
    /// # use checkito::*;
    /// // A generator for the square roots of perfect squares.
    /// let roots = (0..100).filter_map(|x| {
    ///     let sqrt = (x as f64).sqrt();
    ///     if sqrt.fract() == 0.0 { Some(sqrt as i32) } else { None }
    /// });
    ///
    /// roots.check(|x: Option<i32>| assert!(x.is_some()));
    /// ```
    fn filter_map<T, F: Fn(Self::Item) -> Option<T> + Clone>(self, filter: F) -> FilterMap<Self, F>
    where
        Self: Sized,
    {
        prelude::filter_map(self, filter, RETRIES)
    }

    /// Creates a new generator that filters and maps, with a configurable number of retries.
    ///
    /// See [`Generate::filter_map`] for more details.
    fn filter_map_with<T, F: Fn(Self::Item) -> Option<T> + Clone>(
        self,
        retries: usize,
        filter: F,
    ) -> FilterMap<Self, F>
    where
        Self: Sized,
    {
        prelude::filter_map(self, filter, retries)
    }

    /// Creates a new generator by applying a function to the output of `self`,
    /// and then flattening the result.
    ///
    /// This is analogous to [`Iterator::flat_map`]. It's useful for creating a new
    /// generator that depends on the value of a previous one.
    ///
    /// # Examples
    /// ```
    /// # use checkito::*;
    /// // A generator that first picks a length, then creates a vector of that length.
    /// let vec_gen = (0..10).flat_map(|len| (0..100).collect_with(len));
    ///
    /// vec_gen.check(|v: Vec<i32>| assert!(v.len() < 10));
    /// ```
    fn flat_map<G: Generate, F: Fn(Self::Item) -> G + Clone>(self, map: F) -> Flatten<Map<Self, F>>
    where
        Self: Sized,
    {
        prelude::flat_map(self, map)
    }

    /// Flattens a generator of generators.
    ///
    /// If you have a generator that produces other generators (`Generate<Item = impl Generate>`),
    /// this combinator will "unwrap" it, producing values from the inner generator.
    ///
    /// This is particularly important for recursive generation, as it increases the
    /// `depth` parameter in the [`State`], which is used by [`Generate::dampen`] to
    /// control the size of recursive structures.
    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
        Self::Item: Generate,
    {
        prelude::flatten(self)
    }

    /// Creates a new generator that randomly chooses one of a set of generators.
    ///
    /// The `any` combinator can be applied to tuples of generators or slices/vectors
    /// of generators. The resulting value will be wrapped in an `orn::Or` type to
    /// represent the choice, which can be simplified using [`Generate::unify`].
    ///
    /// # Examples
    ///
    /// Choosing between a `char` or an `i32`:
    /// ```
    /// # use checkito::*;
    /// let choice = (('a'..'z'), (0..100)).any();
    ///
    /// // The result is `orn::Or<char, i32>`.
    /// choice.check(|either| {
    ///     match either {
    ///         orn::Or::T0(c) => assert!(c.is_alphabetic()),
    ///         orn::Or::T1(i) => assert!(i < 100),
    ///     }
    /// });
    /// ```
    ///
    /// To perform a weighted choice, see [`Weight`](crate::state::Weight).
    fn any(self) -> Any<Self>
    where
        Self: Sized,
    {
        prelude::any(self)
    }

    /// Creates a generator that produces a fixed-size array.
    ///
    /// It repeatedly calls `self` `N` times to fill the array.
    ///
    /// # Examples
    /// ```
    /// # use checkito::*;
    /// let array_gen = (0..10).array::<4>();
    ///
    /// array_gen.check(|arr: [i32; 4]| assert_eq!(arr.len(), 4));
    /// ```
    fn array<const N: usize>(self) -> Array<Self, N>
    where
        Self: Sized,
    {
        prelude::array(self)
    }

    /// Creates a generator that produces a collection of a default size.
    ///
    /// This is a shorthand for [`Generate::collect_with`] with a default size range.
    /// The resulting collection type is inferred from the context.
    ///
    /// # Examples
    /// ```
    /// # use checkito::*;
    /// let string_gen = letter().collect::<String>();
    /// let vec_gen = (0..10).collect::<Vec<_>>();
    /// ```
    fn collect<F: FromIterator<Self::Item>>(
        self,
    ) -> Collect<Self, Range<Usize<0>, Usize<COLLECTS>>, F>
    where
        Self: Sized,
    {
        prelude::collect(self, Constant::VALUE)
    }

    /// Creates a generator that produces a collection of a specified size.
    ///
    /// The `count` parameter can be any generator that produces a `usize` or a range,
    /// which determines the number of items in the final collection.
    fn collect_with<C: Count, F: FromIterator<Self::Item>>(self, count: C) -> Collect<Self, C, F>
    where
        Self: Sized,
    {
        prelude::collect(self, count)
    }

    /// Creates a generator that modifies the `size` parameter for subsequent generation.
    ///
    /// The `size` is a value in the range `[0.0..=1.0]` that guides generators to
    /// produce "smaller" or "larger" values. This combinator allows you to provide a
    /// function to transform the current `size`.
    ///
    /// # Examples
    ///
    /// Forcing a generator to always produce "large" values:
    /// ```
    /// # use checkito::*;
    /// // This will generate vectors that are likely to be long.
    /// let long_vecs = (0..100).collect().size(|_, _| 1.0);
    /// ```
    fn size<S: Into<Sizes>, F: Fn(Sizes) -> S>(self, map: F) -> Size<Self, F>
    where
        Self: Sized,
    {
        prelude::size(self, map)
    }

    /// Dampens the `size` of generation, typically for recursive structures.
    ///
    /// As recursion gets deeper (tracked by [`Generate::flatten`]), this combinator
    /// reduces the `size`, encouraging base cases to be generated and preventing
    /// infinite growth.
    ///
    /// This is a shorthand for [`Generate::dampen_with`] with default parameters.
    fn dampen(self) -> Dampen<Self>
    where
        Self: Sized,
    {
        prelude::dampen(self, 1.0, 8, 8192)
    }

    /// Dampens the `size` with configurable parameters.
    ///
    /// See [`Generate::dampen`] and [`Generate::size`] for more details.
    /// - `pressure`: How fast the `size` is reduced as `depth` increases.
    /// - `deepest`: The `depth` at which `size` becomes `0`.
    /// - `limit`: The total number of `depth` increases before `size` becomes `0`.
    fn dampen_with(self, pressure: f64, deepest: usize, limit: usize) -> Dampen<Self>
    where
        Self: Sized,
    {
        prelude::dampen(self, pressure, deepest, limit)
    }

    /// Creates a generator whose values are not shrunk.
    ///
    /// If a test fails, a value produced by `keep` will remain constant, while other
    /// inputs are shrunk. This is useful for isolating a failure to a specific input.
    ///
    /// # Examples
    /// ```
    /// # use checkito::*;
    /// // If this test fails, `x` will be shrunk but `y` will remain the same.
    /// #[check((0..100), (0..100).keep())]
    /// fn my_test(x: i32, y: i32) {
    ///     // ...
    /// }
    /// ```
    fn keep(self) -> Keep<Self>
    where
        Self: Sized,
    {
        prelude::keep(self)
    }

    /// Unifies a generator of a "choice" type (like `orn::Or` or `Result`) into a
    /// single type.
    ///
    /// This is often used after [`Generate::any`] to simplify the resulting type.
    ///
    /// # Examples
    /// ```
    /// # use checkito::*;
    /// // `any` produces `orn::Or<i16, u8>`, but `unify` converts both to `i32`.
    /// let unified = ((-100i16..0), (0..100u8)).any().unify::<i32>();
    ///
    /// unified.check(|x: i32| assert!(x < 100));
    /// ```
    fn unify<T>(self) -> Unify<Self, T>
    where
        Self: Sized,
    {
        prelude::unify(self)
    }

    /// Creates a new generator that converts the output of `self` using [`From`].
    ///
    /// This is a convenient way to change a value's type if a `From` implementation
    /// is available.
    ///
    /// # Examples
    /// ```
    /// # use checkito::*;
    /// // Generates a `u8` and then converts it to an `i32`.
    /// let generator = (0..=255u8).convert::<i32>();
    ///
    /// generator.check(|x: i32| assert!(x >= 0 && x <= 255));
    /// ```
    fn convert<T: From<Self::Item>>(self) -> Convert<Self, T>
    where
        Self: Sized,
    {
        prelude::convert(self)
    }
}

impl<G: Generate + ?Sized> Generate for &G {
    type Item = G::Item;
    type Shrink = G::Shrink;

    const CARDINALITY: Option<u128> = G::CARDINALITY;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn cardinality(&self) -> Option<u128> {
        G::cardinality(self)
    }
}

impl<G: Generate + ?Sized> Generate for &mut G {
    type Item = G::Item;
    type Shrink = G::Shrink;

    const CARDINALITY: Option<u128> = G::CARDINALITY;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn cardinality(&self) -> Option<u128> {
        G::cardinality(self)
    }
}
