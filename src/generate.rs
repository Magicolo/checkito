use crate::{
    any::Any,
    array::Array,
    boxed::Boxed,
    collect::Collect,
    convert::Convert,
    dampen::Dampen,
    filter::Filter,
    filter_map::FilterMap,
    flatten::Flatten,
    keep::Keep,
    map::Map,
    random::{self, Random},
    shrink::Shrink,
    size::Size,
    unify::Unify,
};
use core::{
    iter::{FromIterator, FusedIterator},
    marker::PhantomData,
    ops::{self, RangeInclusive},
};

pub(crate) const COUNT: usize = 1024;

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

    /// Primary method of this trait. It generates a [`Shrink`] instance that
    /// will be able to produce values of type [`Generate::Item`] and shrink
    /// itself.
    fn generate(&self, state: &mut State) -> Self::Shrink;

    /// Returns true if the generator will always produce the same item.
    /// This is used in some optimizations to prevent redundant generations.
    fn constant(&self) -> bool {
        false
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
        Boxed<Self::Item>: Generate,
    {
        Boxed::new(self)
    }

    /// Maps generated [`Generate::Item`] to an arbitrary type `T` using the
    /// provided function `F`.
    fn map<T, F: Fn(Self::Item) -> T>(self, map: F) -> Map<Self, F>
    where
        Self: Sized,
        Map<Self, F>: Generate,
    {
        Map(map, self)
    }

    /// Same as [`Generate::filter_with`] but with a predefined number of
    /// `retries`.
    fn filter<F: Fn(&Self::Item) -> bool>(self, filter: F) -> Filter<Self, F>
    where
        Self: Sized,
        Filter<Self, F>: Generate,
    {
        self.filter_with(COUNT, filter)
    }

    /// Generates many [`Generate::Item`] with an increasingly large `size`
    /// until the filter function `F` is satisfied, up to the maximum number
    /// of `retries`.
    ///
    /// Since this [`Generate`] implementation is not guaranteed to succeed,
    /// the item type is changed to a [`Option<Generate::Item>`]
    /// where a [`None`] represents the failure to satisfy the filter.
    fn filter_with<F: Fn(&Self::Item) -> bool>(self, retries: usize, filter: F) -> Filter<Self, F>
    where
        Self: Sized,
        Filter<Self, F>: Generate,
    {
        Filter::new(self, filter, retries)
    }

    /// Same as [`Generate::filter_map_with`] but with a predefined number of
    /// `retries`.
    fn filter_map<T, F: Fn(Self::Item) -> Option<T>>(self, map: F) -> FilterMap<Self, F>
    where
        Self: Sized,
        FilterMap<Self, F>: Generate,
    {
        self.filter_map_with(COUNT, map)
    }

    /// Combines [`Generate::map`] and [`Generate::filter`] in a single
    /// [`Generate`] implementation where the map function is considered to
    /// satisfy the filter when a [`Some(T)`] is produced.
    fn filter_map_with<T, F: Fn(Self::Item) -> Option<T>>(
        self,
        retries: usize,
        map: F,
    ) -> FilterMap<Self, F>
    where
        Self: Sized,
        FilterMap<Self, F>: Generate,
    {
        FilterMap::new(self, map, retries)
    }

    /// Combines [`Generate::map`] and [`Generate::flatten`] in a single
    /// [`Generate`] implementation.
    fn flat_map<G: Generate, F: Fn(Self::Item) -> G>(self, map: F) -> Flatten<Map<Self, F>>
    where
        Self: Sized,
        Map<Self, F>: Generate<Item = G>,
        Flatten<Map<Self, F>>: Generate,
    {
        self.map(map).flatten()
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
    /// through it. Implementations such as [`lazy`](crate::lazy)
    /// and [`Generate::flat_map`] use it.
    ///
    /// The `depth` is particularly useful to limit the amount of recursion that
    /// happens for structures that potentially explode exponentially as the
    /// recursion depth increases (think of a tree structure).
    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
        Self::Item: Generate,
        Flatten<Self>: Generate,
    {
        Flatten(self)
    }

    /// For a type `T` where [`Any<T>`] implements [`Generate`], the behavior
    /// of the generation changes from *generate all* of my components to
    /// *generate one* of my components chosen randomly. It is implemented
    /// for tuples, slices, arrays, [`Vec<T>`] and a few other collections.
    ///
    /// The random selection can be controlled by wrapping each element of a
    /// supported collection in a [`any::Weight`](crate::any::Weight), which
    /// will inform the [`Generate`] implementation to perform a weighted
    /// random between elements of the collection.
    fn any(self) -> Any<Self>
    where
        Self: Sized,
        Any<Self>: Generate,
    {
        Any(self)
    }

    /// Generates `N` items and fills an array with it.
    fn array<const N: usize>(self) -> Array<Self, N>
    where
        Self: Sized,
        Array<Self, N>: Generate,
    {
        Array(self)
    }

    /// Same as [`Generate::collect_with`] but with a predefined `count`.
    fn collect<F: FromIterator<Self::Item>>(self) -> Collect<Self, RangeInclusive<usize>, F>
    where
        Self: Sized,
        Collect<Self, RangeInclusive<usize>, F>: Generate,
    {
        Collect::new(self)
    }

    /// Generates a variable number of items based on the provided `count`
    /// [`Generate`] and then builds a value of type `F` based on its
    /// implementation of [`FromIterator`].
    fn collect_with<C: Generate<Item = usize>, F: FromIterator<Self::Item>>(
        self,
        count: C,
    ) -> Collect<Self, C, F>
    where
        Self: Sized,
        Collect<Self, C, F>: Generate,
    {
        Collect::new_with(self, count, None)
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
    fn size<F: Fn(f64) -> f64>(self, map: F) -> Size<Self, F>
    where
        Self: Sized,
        Size<Self, F>: Generate,
    {
        Size(self, map)
    }

    /// Same as [`Generate::dampen_with`] but with predefined arguments.
    fn dampen(self) -> Dampen<Self>
    where
        Self: Sized,
        Dampen<Self>: Generate,
    {
        self.dampen_with(1.0, 8, 8192)
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
        Dampen<Self>: Generate,
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
        Keep<Self>: Generate,
    {
        Keep(self)
    }

    fn unify<T>(self) -> Unify<Self, T>
    where
        Self: Sized,
        Unify<Self, T>: Generate,
    {
        Unify(PhantomData, self)
    }

    fn convert<T>(self) -> Convert<Self, T>
    where
        Self: Sized,
        Convert<Self, T>: Generate,
    {
        Convert(PhantomData, self)
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

impl<G: Generate + ?Sized> Generate for &G {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn constant(&self) -> bool {
        G::constant(self)
    }
}

impl<G: Generate + ?Sized> Generate for &mut G {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn constant(&self) -> bool {
        G::constant(self)
    }
}
