use crate::{
    any::Any, array::Array, collect::Collect, filter::Filter, filter_map::FilterMap,
    flatten::Flatten, keep::Keep, map::Map, primitive::Range, sample::Sample, shrink::Shrink,
    size::Size, tuples,
};
use fastrand::Rng;
use std::iter::FromIterator;

#[derive(Clone, Debug)]
pub struct State {
    pub size: f64,
    pub seed: u64,
    pub random: Rng,
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

    fn sample(&self, count: usize) -> Sample<Self>
    where
        Self: Sized,
    {
        Sample::new(self, count, None)
    }
}

impl State {
    pub fn new(index: usize, count: usize, seed: u64) -> Self {
        Self {
            size: (index as f64 / count as f64 * 1.1).min(1.),
            seed,
            random: Rng::with_seed(seed),
        }
    }
}

impl<G: FullGenerate> FullGenerate for &G {
    type Item = G::Item;
    type Generate = G::Generate;
    fn generator() -> Self::Generate {
        G::generator()
    }
}

impl<G: FullGenerate> FullGenerate for &mut G {
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

impl<G: Generate> Generate for &G {
    type Item = G::Item;
    type Shrink = G::Shrink;
    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        (&**self).generate(state)
    }
}

impl<G: Generate> Generate for &mut G {
    type Item = G::Item;
    type Shrink = G::Shrink;
    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        (&**self).generate(state)
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
