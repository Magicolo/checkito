use crate::{
    any::Any,
    array::Array,
    boxed,
    collect::Collect,
    dampen::Dampen,
    filter::Filter,
    filter_map::FilterMap,
    flatten::Flatten,
    keep::Keep,
    map::Map,
    primitive::Range,
    random::{self, Random},
    shrink::Shrinker,
    size::Size,
};
use core::{
    iter::{FromIterator, FusedIterator},
    ops,
};

const COUNT: usize = 1024;

#[derive(Clone, Debug)]
pub struct State {
    seed: u64,
    pub(crate) size: (f64, f64),
    pub(crate) limit: u32,
    pub(crate) depth: u32,
    random: Random,
}

#[derive(Debug, Clone)]
pub struct States {
    indices: ops::Range<usize>,
    count: usize,
    size: ops::Range<f64>,
    seed: u64,
}
/// When implemented for a type `T`, this allows to retrieve a generator for `T`
/// that does not require any parameter. It should be implemented for any type

/// that has a canonical way to be generated. To provide a generator with
/// parameters, see [`IntoGenerator`].
///
/// For example, this trait is implemented for all non-pointer primitive types
/// and for some standard types (such as [`Option<T>`] and [`Result<T, E>`]).
pub trait FullGenerator {
    type Item;
    type FullGen: Generator<Item = Self::Item>;
    fn full_gen() -> Self::FullGen;
}

/// When implemented for a type `T`, this allows to retrieve a generate using
/// the values in `T`, similar to the [`Into<T>`] trait.
pub trait IntoGenerator {
    type Item;
    type IntoGen: Generator<Item = Self::Item>;
    fn into_gen(self) -> Self::IntoGen;
}

#[must_use = "generators do nothing until used"]
pub trait Generator {
    type Item;
    type Shrink: Shrinker<Item = Self::Item>;

    /// Primary method of this trait. It generates a [`Shrink`] instance that
    /// will be able to produce values of type [`Generator::Item`] and shrink
    /// itself.
    fn generate(&self, state: &mut State) -> Self::Shrink;

    /// Returns true if the generator will always produce the same item.
    /// This is used in some optimizations to prevent redundant generations.
    fn constant(&self) -> bool {
        false
    }

    /// Wraps `self` in a boxed [`Generator`]. This is notably relevant for
    /// recursive [`Generator`] implementations where the type would
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
    /// fn node() -> impl Generator<Item = Node> {
    ///     (
    ///         with(|| Node::Leaf),
    ///         // Without [`Generator::boxed`], this type would be infinite.
    ///         // Without [`Generator::lazy`], the stack would overflow.
    ///         // Without [`Generator::dampen`], the tree would grow exponentially.
    ///         lazy(node).collect().map(Node::Branch).dampen().boxed(),
    ///     )
    ///         .any()
    ///         .map(|or| or.into())
    /// }
    ///
    /// fn choose(choose: bool) -> impl Generator<Item = char> {
    ///     if choose {
    ///         // Without [`Generator::boxed`], the `if/else` branches would not have the same type.
    ///         letter().boxed()
    ///     } else {
    ///         digit().boxed()
    ///     }
    /// }
    /// ```
    fn boxed(self) -> boxed::Gen<Self::Item>
    where
        Self: Sized + 'static,
    {
        boxed::Gen::new(self)
    }

    /// Maps generated [`Generator::Item`] to an arbitrary type `T` using the
    /// provided function `F`.
    fn map<T, F: Fn(Self::Item) -> T + Clone>(self, map: F) -> Map<Self, F>
    where
        Self: Sized,
    {
        Map::new(self, map)
    }

    /// Same as [`Generator::filter_with`] but with a predefined number of
    /// `retries`.
    fn filter<F: Fn(&Self::Item) -> bool + Clone>(self, filter: F) -> Filter<Self, F>
    where
        Self: Sized,
    {
        self.filter_with(COUNT, filter)
    }

    /// Generates many [`Generator::Item`] with an increasingly large `size`
    /// until the filter function `F` is satisfied, up to the maximum number
    /// of `retries`.
    ///
    /// Since this [`Generator`] implementation is not guaranteed to succeed,
    /// the item type is changed to a [`Option<Generator::Item>`]
    /// where a [`None`] represents the failure to satisfy the filter.
    fn filter_with<F: Fn(&Self::Item) -> bool + Clone>(
        self,
        retries: usize,
        filter: F,
    ) -> Filter<Self, F>
    where
        Self: Sized,
    {
        Filter::new(self, filter, retries)
    }

    /// Same as [`Generator::filter_map_with`] but with a predefined number of
    /// `retries`.
    fn filter_map<T, F: Fn(Self::Item) -> Option<T> + Clone>(self, map: F) -> FilterMap<Self, F>
    where
        Self: Sized,
    {
        self.filter_map_with(COUNT, map)
    }

    /// Combines [`Generator::map`] and [`Generator::filter`] in a single
    /// [`Generator`] implementation where the map function is considered to
    /// satisfy the filter when a [`Some(T)`] is produced.
    fn filter_map_with<T, F: Fn(Self::Item) -> Option<T> + Clone>(
        self,
        retries: usize,
        map: F,
    ) -> FilterMap<Self, F>
    where
        Self: Sized,
    {
        FilterMap::new(self, map, retries)
    }

    /// Combines [`Generator::map`] and [`Generator::flatten`] in a single
    /// [`Generator`] implementation.
    fn flat_map<G: Generator, F: Fn(Self::Item) -> G + Clone>(self, map: F) -> Flatten<Map<Self, F>>
    where
        Self: Sized,
    {
        self.map(map).flatten()
    }

    /// Flattens the [`Generator::Item`], assuming that it implements
    /// [`Generator`]. The resulting item type is `<Generator::Item as
    /// Generator>::Item`.
    ///
    /// Additionally, the call to [`Generator::generate`] to the inner
    /// [`Generator`] implementation will have its `depth` increased by `1`.
    /// The `depth` is a value used by other [`Generator`] implementations (such
    /// as [`Generator::size`] and [`Generator::dampen`]) to alter the `size`
    /// of generated items. It tries to represent the recursion depth since it
    /// is expected that recursive [`Generator`] instances will need to go
    /// through it. Implementations such as [`lazy`](crate::lazy)
    /// and [`Generator::flat_map`] use it.
    ///
    /// The `depth` is particularly useful to limit the amount of recursion that
    /// happens for structures that potentially explode exponentially as the
    /// recursion depth increases (think of a tree structure).
    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
        Self::Item: Generator,
    {
        Flatten(self)
    }

    /// For a type `T` where [`Any<T>`] implements [`Generator`], the behavior
    /// of the generation changes from *generate all* of my components to
    /// *generate one* of my components chosen randomly. It is implemented
    /// for tuples, slices, arrays, [`Vec<T>`] and a few other collections.
    ///
    /// The random selection can be controlled by wrapping each element of a
    /// supported collection in a [`any::Weight`](crate::any::Weight), which
    /// will inform the [`Generator`] implementation to perform a weighted
    /// random between elements of the collection.
    fn any(self) -> Any<Self>
    where
        Self: Sized,
    {
        Any(self)
    }

    /// Generates `N` items and fills an array with it.
    fn array<const N: usize>(self) -> Array<Self, N>
    where
        Self: Sized,
    {
        Array(self)
    }

    /// Same as [`Generator::collect_with`] but with a predefined `count`.
    fn collect<F: FromIterator<Self::Item>>(self) -> Collect<Self, Range<usize>, F>
    where
        Self: Sized,
    {
        self.collect_with((..COUNT).into_gen())
    }

    /// Generates a variable number of items based on the provided `count`
    /// [`Generator`] and then builds a value of type `F` based on its
    /// implementation of [`FromIterator`].
    fn collect_with<C: Generator<Item = usize>, F: FromIterator<Self::Item>>(
        self,
        count: C,
    ) -> Collect<Self, C, F>
    where
        Self: Sized,
    {
        Collect::new(self, count)
    }

    /// Maps the current `size` of the generation process to a different one.
    /// The `size` is a value in the range `[0.0..1.0]` that represents *how
    /// big* the generated items are based on the generator's constraints. The
    /// generation process will initially try to produce *small* items and
    /// then move on to *bigger* ones. Note that the `size` does not
    /// guarantee a *small* or *big* generated item since [`Generator`]
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
    fn size<F: Fn(f64) -> f64>(self, map: F) -> Size<Self, F>
    where
        Self: Sized,
    {
        Size(self, map)
    }

    /// Same as [`Generator::dampen_with`] but with predefined arguments.
    fn dampen(self) -> Dampen<Self>
    where
        Self: Sized,
    {
        self.dampen_with(1.0, 8, 8192)
    }

    /// Dampens the `size` (see [`Generator::size`] for more information about
    /// `size`) as items are generated.
    /// - The `pressure` can be thought of as *how fast* will the `size` be
    ///   reduced as the `depth` increases (see [`Generator::flatten`] for more
    ///   information about `depth`).
    /// - The `deepest` will set the `size` to `0` when the `depth` is `>=` than
    ///   it.
    /// - The `limit` will set the `size` to `0` after the number of times that
    ///   the `depth` increased is `>=` than it.
    ///
    /// This [`Generator`] implementation is particularly useful to limit the
    /// amount of recursion that happens for structures that are infinite
    /// and potentially explode exponentially as the recursion depth increases
    /// (think of a tree structure).
    fn dampen_with(self, pressure: f64, deepest: usize, limit: usize) -> Dampen<Self>
    where
        Self: Sized,
    {
        assert!(pressure.is_finite());
        assert!(pressure >= 0.0);
        Dampen {
            pressure,
            deepest,
            limit,
            generator: self,
        }
    }

    /// Keeps the generated items intact through the shrinking process (i.e.
    /// *un-shrinked*).
    fn keep(self) -> Keep<Self>
    where
        Self: Sized,
    {
        Keep(self)
    }
}

impl State {
    pub(crate) fn new(index: usize, count: usize, size: ops::Range<f64>, seed: u64) -> Self {
        Self {
            size: self::size(index, count, size),
            depth: 0,
            limit: 0,
            seed,
            random: Random::new(seed.wrapping_add(index as _)),
        }
    }

    pub const fn size(&self) -> f64 {
        self.size.0
    }

    pub const fn seed(&self) -> u64 {
        self.seed
    }

    pub fn random(&mut self) -> &mut Random {
        &mut self.random
    }
}

impl States {
    pub fn new(count: usize, size: ops::Range<f64>, seed: Option<u64>) -> Self {
        Self {
            indices: 0..count,
            count,
            size,
            seed: seed.unwrap_or_else(random::seed),
        }
    }
}

impl Iterator for States {
    type Item = State;

    fn next(&mut self) -> Option<Self::Item> {
        Some(State::new(
            self.indices.next()?,
            self.count,
            self.size.clone(),
            self.seed,
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }

    fn count(self) -> usize {
        self.indices.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        Some(State::new(
            self.indices.nth(n)?,
            self.count,
            self.size.clone(),
            self.seed,
        ))
    }

    fn last(mut self) -> Option<Self::Item> {
        Some(State::new(
            self.indices.next()?,
            self.count,
            self.size.clone(),
            self.seed,
        ))
    }
}

impl ExactSizeIterator for States {
    fn len(&self) -> usize {
        self.indices.len()
    }
}

impl DoubleEndedIterator for States {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some(State::new(
            self.indices.next_back()?,
            self.count,
            self.size.clone(),
            self.seed,
        ))
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        Some(State::new(
            self.indices.nth_back(n)?,
            self.count,
            self.size.clone(),
            self.seed,
        ))
    }
}

impl FusedIterator for States {}

pub(crate) fn size(index: usize, count: usize, mut size: ops::Range<f64>) -> (f64, f64) {
    size.start = size.start.clamp(0.0, 1.0);
    size.end = size.end.clamp(0.0, 1.0);

    if count <= 1 {
        (size.end, size.end)
    } else {
        let range = size.end - size.start;
        assert!(range >= 0.0);
        assert!(index <= count);
        // This size calculation ensures that 25% of samples are fully sized.
        let ratio = (index as f64 / count as f64 * 1.25).clamp(0.0, 1.0);
        (size.start + ratio * range, size.end)
    }
}

impl<G: FullGenerator + ?Sized> FullGenerator for &G {
    type FullGen = G::FullGen;
    type Item = G::Item;

    fn full_gen() -> Self::FullGen {
        G::full_gen()
    }
}

impl<G: FullGenerator + ?Sized> FullGenerator for &mut G {
    type FullGen = G::FullGen;
    type Item = G::Item;

    fn full_gen() -> Self::FullGen {
        G::full_gen()
    }
}

impl<G: IntoGenerator + Clone> IntoGenerator for &G {
    type IntoGen = G::IntoGen;
    type Item = G::Item;

    fn into_gen(self) -> Self::IntoGen {
        self.clone().into_gen()
    }
}

impl<G: IntoGenerator + Clone> IntoGenerator for &mut G {
    type IntoGen = G::IntoGen;
    type Item = G::Item;

    fn into_gen(self) -> Self::IntoGen {
        self.clone().into_gen()
    }
}

impl<G: Generator + ?Sized> Generator for &G {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn constant(&self) -> bool {
        G::constant(self)
    }
}

impl<G: Generator + ?Sized> Generator for &mut G {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn constant(&self) -> bool {
        G::constant(self)
    }
}