use crate::{
    COLLECTS, RETRIES,
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
};
use core::iter::FromIterator;

/// When implemented for a type `T`, this allows to retrieve a generator for `T`
/// that does not require any parameter. It should be implemented for any type
/// that has a canonical way to be generated.
///
/// For example, this trait is implemented for all non-pointer primitive types
/// and for some standard types (such as [`Option<T>`] and [`Result<T, E>`]).
pub trait FullGenerate {
    type Item;
    type Generator: Generate<Item = Self::Item>;
    fn generator() -> Self::Generator;
}

#[must_use = "generators do nothing until used"]
pub trait Generate {
    type Item;
    type Shrink: Shrink<Item = Self::Item>;

    /// The static cardinality of the generated values. This value can be
    /// thought as the following: for the set of all generators of type
    /// [`Self`], how large is the set of all possible [`Self::Item`] that they
    /// could generate. If the cardinality of that set can not be determined
    /// or is too large to fit in a [`usize`], set it to [`None`].
    const CARDINALITY: Option<u128>;

    /// Primary method of this trait. It generates a [`Shrink`] instance that
    /// will be able to produce values of type [`Generate::Item`] and shrink
    /// itself.
    fn generate(&self, state: &mut State) -> Self::Shrink;

    /// Returns the dynamic cardinality of the generated values. This value can
    /// be thought as the following: for this specific generator of type
    /// [`Self`], how large is the set of all possible [`Self::Item`] that it
    /// could generate. If the cardinality of that set can not be determined
    /// or is too large to fit in a [`usize`], set it to [`None`].
    fn cardinality(&self) -> Option<u128> {
        Self::CARDINALITY
    }

    /// Wraps `self` in a boxed [`Generate`]. This is notably relevant for
    /// recursive [`Generate`] implementations where the type would
    /// otherwise be infinite.
    ///
    /// # Examples
    /// ```
    /// use checkito::*;
    ///
    /// enum Node {
    ///     Leaf,
    ///     Branch(Vec<Node>),
    /// }
    ///
    /// fn node() -> impl Generate<Item = Node> {
    ///     (
    ///         with(|| Node::Leaf),
    ///         // Without [`Generate::boxed`], this type would be infinite.
    ///         // Without [`Generate::lazy`], the stack would overflow.
    ///         // Without [`Generate::dampen`], the tree would grow exponentially.
    ///         lazy(node).collect().map(Node::Branch).dampen().boxed(),
    ///     )
    ///         .any()
    ///         .unify()
    /// }
    ///
    /// fn choose(choose: bool) -> impl Generate<Item = char> {
    ///     if choose {
    ///         // Without [`Generate::boxed`], the `if/else` branches would not have the same type.
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

    /// Maps generated [`Generate::Item`] to an arbitrary type `T` using the
    /// provided function `F`.
    fn map<T, F: Fn(Self::Item) -> T + Clone>(self, map: F) -> Map<Self, F>
    where
        Self: Sized,
    {
        prelude::map(self, map)
    }

    /// Same as [`Generate::filter_with`] but with a default `retries` of
    /// [`RETRIES`].
    fn filter<F: Fn(&Self::Item) -> bool + Clone>(self, filter: F) -> Filter<Self, F>
    where
        Self: Sized,
    {
        prelude::filter(self, filter, RETRIES)
    }

    /// Generates many [`Generate::Item`] with an increasingly large `size`
    /// until the filter function `F` is satisfied, up to the maximum number
    /// of `retries`.
    ///
    /// Since this [`Generate`] implementation is not guaranteed to succeed,
    /// the item type is changed to a [`Option<Generate::Item>`]
    /// where a [`None`] represents the failure to satisfy the filter.
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

    /// Same as [`Generate::filter_map_with`] but with a predefined number of
    /// `retries`.
    fn filter_map<T, F: Fn(Self::Item) -> Option<T> + Clone>(self, filter: F) -> FilterMap<Self, F>
    where
        Self: Sized,
    {
        prelude::filter_map(self, filter, RETRIES)
    }

    /// Combines [`Generate::map`] and [`Generate::filter`] in a single
    /// [`Generate`] implementation where the map function is considered to
    /// satisfy the filter when a [`Some<T>`] is produced.
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

    /// Combines [`Generate::map`] and [`Generate::flatten`] in a single
    /// [`Generate`] implementation.
    fn flat_map<G: Generate, F: Fn(Self::Item) -> G + Clone>(self, map: F) -> Flatten<Map<Self, F>>
    where
        Self: Sized,
    {
        prelude::flat_map(self, map)
    }

    /// Flattens the [`Generate::Item`], assuming that it implements
    /// [`Generate`]. The resulting item type is `<Generate::Item as
    /// Generate>::Item`.
    ///
    /// Additionally, the call to [`Generate::generate`] to the inner
    /// [`Generate`] implementation will have its `depth` increased by `1`.
    /// The `depth` is a value used by other [`Generate`] implementations (such
    /// as [`Generate::size`] and [`Generate::dampen`]) to alter the `size`
    /// of generated items. It tries to represent the recursion depth since it
    /// is expected that recursive [`Generate`] instances will need to go
    /// through it. Implementations such as [`lazy`](prelude::lazy)
    /// and [`Generate::flat_map`] use it.
    ///
    /// The `depth` is particularly useful to limit the amount of recursion that
    /// happens for structures that potentially explode exponentially as the
    /// recursion depth increases (think of a tree structure).
    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
        Self::Item: Generate,
    {
        prelude::flatten(self)
    }

    /// For a type `T` where [`Any<T>`](crate::any::Any) implements
    /// [`Generate`], the behavior of the generation changes from *generate
    /// all* of my components to *generate one* of my components chosen
    /// randomly. It is implemented for tuples, slices, arrays, [`Vec<T>`]
    /// and a few other collections.
    ///
    /// The random selection can be controlled by wrapping each element of a
    /// supported collection in a [`any::Weight`](crate::any::Weight), which
    /// will inform the [`Generate`] implementation to perform a weighted
    /// random between elements of the collection.
    fn any(self) -> Any<Self>
    where
        Self: Sized,
    {
        prelude::any(self)
    }

    /// Generates `N` items and fills an array with it.
    fn array<const N: usize>(self) -> Array<Self, N>
    where
        Self: Sized,
    {
        prelude::array(self)
    }

    /// Same as [`Generate::collect_with`] but with a default `count` of
    /// [`COLLECTS`].
    fn collect<F: FromIterator<Self::Item>>(
        self,
    ) -> Collect<Self, Range<Usize<0>, Usize<COLLECTS>>, F>
    where
        Self: Sized,
    {
        prelude::collect(self, Constant::VALUE)
    }

    /// Generates a variable number of items based on the provided `count`
    /// [`Generate`] and then builds a value of type `F` based on its
    /// implementation of [`FromIterator`].
    fn collect_with<C: Count, F: FromIterator<Self::Item>>(self, count: C) -> Collect<Self, C, F>
    where
        Self: Sized,
    {
        prelude::collect(self, count)
    }

    /// Maps the current `size` of the generation process to a different one.
    /// The `size` is a value in the range `[0.0..1.0]` that represents *how
    /// big* the generated items are based on the generator's constraints. The
    /// generation process will initially try to produce *small* items and
    /// then move on to *bigger* ones. Note that the `size` does not
    /// guarantee a *small* or *big* generated item since [`Generate`]
    /// implementations are free to interpret it as they wish, although that
    /// is its intention.
    ///
    /// For example, a *small* number will be close to `0`, a *small* collection
    /// will have its `len()` close to `0`, a *large* [`bool`] will be
    /// `true`, a *large* [`String`] will have many [`char`], etc.
    ///
    /// The provided `map` function is described as such:
    /// - Its first argument is the current `size` in the range `[0.0..1.0]`.
    /// - Its return value will be clamped to the `[0.0..1.0]` range and panic
    ///   if it is infinite or [`f64::NAN`].
    ///
    /// Useful to nullify the sizing of items (`self.size(|_, _| 1.0)` will
    /// always produces items of full `size`) or to attenuate the `size`.
    fn size<S: Into<Sizes>, F: Fn(Sizes) -> S>(self, map: F) -> Size<Self, F>
    where
        Self: Sized,
    {
        prelude::size(self, map)
    }

    /// Same as [`Generate::dampen_with`] but with predefined arguments.
    fn dampen(self) -> Dampen<Self>
    where
        Self: Sized,
    {
        prelude::dampen(self, 1.0, 8, 8192)
    }

    /// Dampens the `size` (see [`Generate::size`] for more information about
    /// `size`) as items are generated.
    /// - The `pressure` can be thought of as *how fast* will the `size` be
    ///   reduced as the `depth` increases (see [`Generate::flatten`] for more
    ///   information about `depth`).
    /// - The `deepest` will set the `size` to `0` when the `depth` is `>=` than
    ///   it.
    /// - The `limit` will set the `size` to `0` after the number of times that
    ///   the `depth` increased is `>=` than it.
    ///
    /// This [`Generate`] implementation is particularly useful to limit the
    /// amount of recursion that happens for structures that are infinite
    /// and potentially explode exponentially as the recursion depth increases
    /// (think of a tree structure).
    fn dampen_with(self, pressure: f64, deepest: usize, limit: usize) -> Dampen<Self>
    where
        Self: Sized,
    {
        prelude::dampen(self, pressure, deepest, limit)
    }

    /// Keeps the generated items intact through the shrinking process (i.e.
    /// *un-shrinked*).
    fn keep(self) -> Keep<Self>
    where
        Self: Sized,
    {
        prelude::keep(self)
    }

    fn unify<T>(self) -> Unify<Self, T>
    where
        Self: Sized,
    {
        prelude::unify(self)
    }

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
