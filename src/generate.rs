use crate::{
    any::Any,
    array::Array,
    boxed,
    check::{Checker, Checks, Error},
    collect::Collect,
    dampen::Dampen,
    filter::Filter,
    filter_map::FilterMap,
    flatten::Flatten,
    keep::Keep,
    map::Map,
    primitive::Range,
    prove::Prove,
    sample::{Sampler, Samples},
    shrink::{All, Shrink},
    size::Size,
    tuples,
};
use fastrand::Rng;
use std::iter::FromIterator;

#[derive(Clone, Debug)]
pub struct State {
    pub(crate) size: f64,
    pub(crate) count: usize,
    pub(crate) depth: usize,
    seed: u64,
    random: Rng,
}

/// When implemented for a type `T`, this allows to retrieve a generator for `T` that does not require any parameter.
/// It should be implemented for any type that has a canonical way to be generated.
/// To provide a generator with parameters, see [`IntoGenerate`].
///
/// For example, this trait is implemented for all non-pointer primitive types and for some standard types (such as [`Option<T>`] amd [`Result<T, E>`]).
pub trait FullGenerate {
    type Item;
    type Generate: Generate<Item = Self::Item>;
    fn generator() -> Self::Generate;
}

/// When implemented for a type `T`, this allows to retrieve a generate using the values in `T`, similar to the [`Into<T>`] trait.
pub trait IntoGenerate {
    type Item;
    type Generate: Generate<Item = Self::Item>;
    fn generator(self) -> Self::Generate;
}

pub trait Generate {
    type Item;
    type Shrink: Shrink<Item = Self::Item>;

    /// Primary method of this trait. It generates a [`Shrink`] instance that will be able to produce values of type
    /// [`Generate::Item`] and shrink itself.
    fn generate(&self, state: &mut State) -> Self::Shrink;

    /// Wraps `self` in a boxed [`Generate`]. This is notably relevant for recursive [`Generate`] implementations where
    /// the type would otherwise be infinite.
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
    ///         lazy(node).collect().map(Node::Branch).boxed()
    ///     )
    ///     .any()
    ///     .map(Unify::unify)
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
    fn boxed(self) -> boxed::Generator<Self::Item>
    where
        Self: Sized + 'static,
        boxed::Generator<Self::Item>: Generate,
    {
        boxed::Generator::new(self)
    }

    /// Maps generated [`Generate::Item`] to an arbitrary type `T` using the provided function `F`.
    fn map<T, F: Fn(Self::Item) -> T>(self, map: F) -> Map<Self, T, F>
    where
        Self: Sized,
        Map<Self, T, F>: Generate,
    {
        Map::new(self, map)
    }

    /// Same as [`Generate::filter_with`] but with a predefined number of `retries`.
    fn filter<F: Fn(&Self::Item) -> bool>(self, filter: F) -> Filter<Self, F>
    where
        Self: Sized,
        Filter<Self, F>: Generate,
    {
        self.filter_with(256, filter)
    }

    /// Generates many [`Generate::Item`] with an increasingly large `size` until the filter function `F` is satisfied, up to
    /// the maximum number of `retries`.
    ///
    /// Since this [`Generate`] implementation is not guaranteed to succeed, the item type is changed to a [`Option<Generate::Item>`]
    /// where a [`None`] represents the failure to satisfy the filter.
    fn filter_with<F: Fn(&Self::Item) -> bool>(self, retries: usize, filter: F) -> Filter<Self, F>
    where
        Self: Sized,
        Filter<Self, F>: Generate,
    {
        Filter::new(self, filter, retries)
    }

    /// Same as [`Generate::filter_map_with`] but with a predefined number of `retries`.
    fn filter_map<T, F: Fn(Self::Item) -> Option<T>>(self, map: F) -> FilterMap<Self, T, F>
    where
        Self: Sized,
        FilterMap<Self, T, F>: Generate,
    {
        self.filter_map_with(256, map)
    }

    /// Combines [`Generate::map`] and [`Generate::filter`] in a single [`Generate`] implementation where the map function
    /// is considered to satisfy the filter when a [`Some(T)`] is produced.
    fn filter_map_with<T, F: Fn(Self::Item) -> Option<T>>(
        self,
        retries: usize,
        map: F,
    ) -> FilterMap<Self, T, F>
    where
        Self: Sized,
        FilterMap<Self, T, F>: Generate,
    {
        FilterMap::new(self, map, retries)
    }

    /// Combines [`Generate::map`] and [`Generate::flatten`] in a single [`Generate`] implementation.
    fn flat_map<G: Generate, F: Fn(Self::Item) -> G>(self, map: F) -> Flatten<Map<Self, G, F>>
    where
        Self: Sized,
        Map<Self, G, F>: Generate<Item = G>,
        Flatten<Map<Self, G, F>>: Generate,
    {
        self.map(map).flatten()
    }

    /// Flattens the [`Generate::Item`], assuming that it implements [`Generate`]. The resulting item type is
    /// `<Generate::Item as Generate>::Item`.
    ///
    /// Additionally, the call to [`Generate::generate`] to the inner [`Generate`] implementation will have its `depth`
    /// increased by `1`. The `depth` is a value used by other [`Generate`] implementations (such as [`Generate::size`] and
    /// [`Generate::dampen`]) to alter the `size` of generated items. It tries to represent the recursion depth since it
    /// is expected that recursive [`Generate`] instances will need to go through it. Implementations such as [`lazy`](crate::lazy)
    /// and [`Generate::flat_map`] use it.
    ///
    /// The `depth` is particularly useful to limit the amount of recursion that happens for
    /// structures that potentially explode exponentially as the recursion depth increases (think of a tree structure).
    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
        Self::Item: Generate,
        Flatten<Self>: Generate,
    {
        Flatten(self)
    }

    /// For a type `T` where [`Any<T>`] implements [`Generate`], the behavior of the generation changes from *generate all* of
    /// my components to *generate one* of my components chosen randomly.
    /// It is implemented for tuples, slices, arrays, [`Vec<T>`] and a few other collections.
    ///
    /// The random selection can be controlled by wrapping each element of a supported collection in a
    /// [`any::Weight`](crate::any::Weight), which will inform the [`Generate`] implementation to perform a weighted random
    /// between elements of the collection.
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
    fn collect<F: FromIterator<Self::Item>>(self) -> Collect<Self, Range<usize>, F>
    where
        Self: Sized,
        Collect<Self, Range<usize>, F>: Generate,
    {
        self.collect_with((..256usize).generator())
    }

    /// Generates a variable number of items based on the provided `count` [`Generate`] and then builds a value of type
    /// `F` based on its implementation of [`FromIterator`].
    fn collect_with<C: Generate<Item = usize>, F: FromIterator<Self::Item>>(
        self,
        count: C,
    ) -> Collect<Self, C, F>
    where
        Self: Sized,
        Collect<Self, C, F>: Generate,
    {
        Collect::new(self, count)
    }

    /// Maps the current `size` of the generation process to a different one. The `size` is a value in the range `[0.0..1.0]`
    /// that represents *how big* the generated items are based on the generator's constraints. The generation process will
    /// initially try to produce *small* items and then move on to *bigger* ones.
    /// Note that the `size` does not guarantee a *small* or *big* generated item since [`Generate`] implementations are free
    /// to interpret it as they wish, although that is its intention.
    ///
    /// For example, a *small* number will be close to `0`, a *small* collection will have its `len()` close to `0`, a *large*
    /// [`bool`] will be `true`, a *large* [`String`] will have many [`char`], etc.
    ///
    /// The provided `map` function is described as such:
    /// - Its first argument is the current `size` in the range `[0.0..1.0]`.
    /// - Its second argument is the current `depth` (see [`Generate::flatten`] for more information about `depth`).
    /// - Its return value will be clamped to the `[0.0..1.0]` range and panic if it is infinite or [`f64::NAN`].
    ///
    /// Useful to nullify the sizing of items (`self.size(|_, _| 1.0)` will always produces items of full `size`) or to
    /// attenuate the `size`.
    fn size<F: Fn(f64, usize) -> f64>(self, map: F) -> Size<Self, F>
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
        Size<Self>: Generate,
    {
        self.dampen_with(1.0, 8, 8192)
    }

    /// Dampens the `size` (see [`Generate::size`] for more information about `size`) as items are generated.
    /// - The `pressure` can be thought of as how fast will the `size` be reduced as the `depth` increases (see [`Generate::flatten`]
    /// for more information about `depth`).
    /// - The `deepest` will set the `size` to `0` when the `depth` is `>=` than it.
    /// - The `limit` will set the `size` to `0` after the number of times that the `depth` increased is `>=` than it.
    ///
    /// This [`Generate`] implementation is particularly useful to limit the amount of recursion that happens for structures
    /// that are infinite and potentially explode exponentially as the recursion depth increases (think of a tree structure).
    fn dampen_with(self, pressure: f64, deepest: usize, limit: usize) -> Dampen<Self>
    where
        Self: Sized,
        Dampen<Self>: Generate,
    {
        debug_assert!(pressure.is_finite());
        debug_assert!(pressure >= 0.0);
        Dampen {
            pressure,
            deepest,
            limit,
            inner: self,
        }
    }

    /// Keeps the generated items intact through the shrinking process (i.e. *un-shrinked*).
    fn keep(self) -> Keep<Self>
    where
        Self: Sized,
        Keep<Self>: Generate,
    {
        Keep(self)
    }

    /// Provides a [`Sampler`] that allows to configure sampling settings and generate samples.
    fn sampler(&self) -> Sampler<Self> {
        Sampler::new(self, None)
    }

    /// Generates `count` random values the are progressively larger in size. For additional sampling settings, see [`Generate::sampler`].
    fn samples(&self, count: usize) -> Samples<Self> {
        self.sampler().samples(count)
    }

    /// Generates a random value of `size` (0.0..=1.0). For additional sampling settings, see [`Generate::sampler`].
    fn sample(&self, size: f64) -> Self::Item {
        self.sampler().sample(size)
    }

    fn checker(&self) -> Checker<Self> {
        Checker::new(self)
    }

    fn checks<P: Prove, F: FnMut(&Self::Item) -> P>(
        &self,
        count: usize,
        check: F,
    ) -> Checks<Self, P, F> {
        self.checker().checks(count, check)
    }

    fn check<P: Prove, F: FnMut(&Self::Item) -> P>(
        &self,
        count: usize,
        check: F,
    ) -> Result<(), Error<Self::Item, P>> {
        for result in self.checks(count, check) {
            result?;
        }
        Ok(())
    }
}

impl State {
    pub fn new(size: f64, seed: Option<u64>) -> Self {
        let random = seed.map_or_else(Rng::new, Rng::with_seed);
        Self {
            size: size.max(0.0).min(1.0),
            depth: 0,
            count: 0,
            seed: random.get_seed(),
            random,
        }
    }

    pub fn from_iteration(index: usize, count: usize, seed: Option<u64>) -> Self {
        // This size calculation ensures that 10% of samples are fully sized.
        if count == 1 {
            Self::new(1.0, seed)
        } else {
            Self::new((index as f64 / count as f64 * 1.1).min(1.), seed)
        }
    }

    pub const fn size(&self) -> f64 {
        self.size
    }

    pub const fn depth(&self) -> usize {
        self.depth
    }

    pub const fn seed(&self) -> u64 {
        self.seed
    }

    pub const fn random(&self) -> &Rng {
        &self.random
    }
}

impl<G: FullGenerate + ?Sized> FullGenerate for &G {
    type Item = G::Item;
    type Generate = G::Generate;
    fn generator() -> Self::Generate {
        G::generator()
    }
}

impl<G: FullGenerate + ?Sized> FullGenerate for &mut G {
    type Item = G::Item;
    type Generate = G::Generate;
    fn generator() -> Self::Generate {
        G::generator()
    }
}

impl<G: IntoGenerate + Clone> IntoGenerate for &G {
    type Item = G::Item;
    type Generate = G::Generate;
    fn generator(self) -> Self::Generate {
        self.clone().generator()
    }
}

impl<G: IntoGenerate + Clone> IntoGenerate for &mut G {
    type Item = G::Item;
    type Generate = G::Generate;
    fn generator(self) -> Self::Generate {
        self.clone().generator()
    }
}

impl<G: Generate + ?Sized> Generate for &G {
    type Item = G::Item;
    type Shrink = G::Shrink;
    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }
}

impl<G: Generate + ?Sized> Generate for &mut G {
    type Item = G::Item;
    type Shrink = G::Shrink;
    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }
}

macro_rules! tuple {
    ($n:ident, $c:tt $(,$p:ident, $t:ident, $i:tt)*) => {
        impl<$($t: FullGenerate,)*> FullGenerate for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type Generate = ($($t::Generate,)*);

            fn generator() -> Self::Generate {
                ($($t::generator(),)*)
            }
        }

        impl<$($t: IntoGenerate,)*> IntoGenerate for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type Generate = ($($t::Generate,)*);

            fn generator(self) -> Self::Generate {
                ($(self.$i.generator(),)*)
            }
        }

        impl<$($t: Generate,)*> Generate for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type Shrink = All<($($t::Shrink,)*)>;

            fn generate(&self, _state: &mut State) -> Self::Shrink {
                All::new(($(self.$i.generate(_state),)*))
            }
        }
    };
}

tuples!(tuple);
