use crate::{
    any::Any,
    array::Array,
    check::{Checker, Checks, Error},
    collect::Collect,
    filter::Filter,
    filter_map::FilterMap,
    flatten::Flatten,
    keep::Keep,
    map::Map,
    primitive::Range,
    sample::{Sampler, Samples},
    shrink::Shrink,
    size::Size,
    tuples, Prove,
};
use fastrand::Rng;
use std::iter::FromIterator;

#[derive(Clone, Debug)]
pub struct State {
    size: f64,
    seed: u64,
    random: Rng,
}

pub trait FullGenerate {
    type Item;
    type Generate: Generate<Item = Self::Item>;
    fn generator() -> Self::Generate;
}

pub trait IntoGenerate {
    type Item;
    type Generate: Generate<Item = Self::Item>;
    fn generator(self) -> Self::Generate;
}

pub trait Generate {
    type Item;
    type Shrink: Shrink<Item = Self::Item>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink);

    fn map<T, F: Fn(Self::Item) -> T>(self, map: F) -> Map<Self, T, F>
    where
        Self: Sized,
        Map<Self, T, F>: Generate,
    {
        Map::generator(self, map)
    }

    fn filter<F: Fn(&Self::Item) -> bool>(
        self,
        iterations: Option<usize>,
        filter: F,
    ) -> Filter<Self, F>
    where
        Self: Sized,
        Filter<Self, F>: Generate,
    {
        Filter::new(self, filter, iterations.unwrap_or(256))
    }

    fn filter_map<T, F: Fn(Self::Item) -> Option<T>>(
        self,
        iterations: Option<usize>,
        map: F,
    ) -> FilterMap<Self, T, F>
    where
        Self: Sized,
        FilterMap<Self, T, F>: Generate,
    {
        FilterMap::new(self, map, iterations.unwrap_or(256))
    }

    fn bind<G: Generate, F: Fn(Self::Item) -> G>(self, bind: F) -> Flatten<Map<Self, G, F>>
    where
        Self: Sized,
        Map<Self, G, F>: Generate<Item = G>,
        Flatten<Map<Self, G, F>>: Generate,
    {
        self.map(bind).flatten()
    }

    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
        Self::Item: Generate,
        Flatten<Self>: Generate,
    {
        Flatten(self)
    }

    fn any(self) -> Any<Self>
    where
        Self: Sized,
        Any<Self>: Generate,
    {
        Any(self)
    }

    fn array<const N: usize>(self) -> Array<Self, N>
    where
        Self: Sized,
        Array<Self, N>: Generate,
    {
        Array(self)
    }

    fn collect<F: FromIterator<Self::Item>>(self) -> Collect<Self, Size<Range<usize>>, F>
    where
        Self: Sized,
        Collect<Self, Size<Range<usize>>, F>: Generate,
    {
        self.collect_with((0..256 as usize).generator())
    }

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

    fn size(self) -> Size<Self>
    where
        Self: Sized,
        Size<Self>: Generate,
    {
        Size(self)
    }

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
            size: size.min(0.0).max(1.0),
            seed: random.get_seed(),
            random,
        }
    }

    pub fn from_iteration(index: usize, count: usize, seed: Option<u64>) -> Self {
        // This size calculation ensures that 10% of samples are fully sized.
        Self::new((index as f64 / count as f64 * 1.1).min(1.), seed)
    }

    pub const fn size(&self) -> f64 {
        self.size
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
    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        G::generate(self, state)
    }
}

impl<G: Generate + ?Sized> Generate for &mut G {
    type Item = G::Item;
    type Shrink = G::Shrink;
    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        G::generate(self, state)
    }
}

macro_rules! tuple {
    ($n:ident, $c:tt) => {
        impl FullGenerate for () {
            type Item = <Self::Generate as Generate>::Item;
            type Generate = ();
            fn generator() -> Self::Generate { () }
        }

        impl IntoGenerate for () {
            type Item = <Self::Generate as Generate>::Item;
            type Generate = ();
            fn generator(self) -> Self::Generate { self }
        }

        impl Generate for () {
            type Item = ();
            type Shrink = ();
            fn generate(&self, _state: &mut State) -> (Self::Item, Self::Shrink) { ((), ()) }
        }

        impl Shrink for () {
            type Item = ();
            fn generate(&self) -> Self::Item { () }
            fn shrink(&mut self) -> Option<Self> { None }
        }
    };
    ($n:ident, $c:tt $(,$p:ident, $t:ident, $i:tt)*) => {
        impl<$($t: FullGenerate,)*> FullGenerate for ($($t,)*) {
            type Item = <Self::Generate as Generate>::Item;
            type Generate = ($($t::Generate,)*);

            fn generator() -> Self::Generate {
                ($($t::generator(),)*)
            }
        }

        impl<$($t: IntoGenerate,)*> IntoGenerate for ($($t,)*) {
            type Item = <Self::Generate as Generate>::Item;
            type Generate = ($($t::Generate,)*);

            fn generator(self) -> Self::Generate {
                ($(self.$i.generator(),)*)
            }
        }

        impl<$($t: Generate,)*> Generate for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type Shrink = ($($t::Shrink,)*);

            fn generate(&self, _state: &mut State) -> (Self::Item, Self::Shrink) {
                let pairs = ($(self.$i.generate(_state),)*);
                (($(pairs.$i.0,)*), ($(pairs.$i.1,)*))
            }
        }

        impl<$($t: Shrink,)*> Shrink for ($($t,)*) {
            type Item = ($($t::Item,)*);

            fn generate(&self) -> Self::Item {
                ($(self.$i.generate(),)*)
            }

            fn shrink(&mut self) -> Option<Self> {
                let mut shrunk = false;
                let shrinks = ($(
                    if shrunk { self.$i.clone() }
                    else {
                        match self.$i.shrink() {
                            Some(shrink) => { shrunk = true; shrink },
                            None => self.$i.clone(),
                        }
                    },
                )*);
                if shrunk { Some(shrinks) } else { None }
            }
        }
    };
}

tuples!(tuple);
