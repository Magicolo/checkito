use crate::{
    generate::{FullGenerate, Generate, IntoGenerate, State},
    primitive::{self, Range},
    same::Same,
    sample::Sample,
    shrink::{All, Shrink},
};
use core::{
    hash::{BuildHasher, Hash},
    marker::PhantomData,
};
use std::{
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque},
    rc::Rc,
    sync::Arc,
};

#[derive(Debug, Default)]
pub struct Collect<I: ?Sized, C, F: ?Sized> {
    _marker: PhantomData<F>,
    count: C,
    minimum: usize,
    inner: I,
}

#[derive(Debug, Default)]
pub struct Generator<I, F: ?Sized> {
    inner: Vec<I>,
    _marker: PhantomData<F>,
}

#[derive(Debug)]
pub struct Shrinker<I, F: ?Sized> {
    inner: Vec<I>,
    index: usize,
    count: primitive::Shrinker<usize>,
    minimum: usize,
    _marker: PhantomData<F>,
}

impl<G: Generate, C: Generate<Item = usize>, F: FromIterator<G::Item>> Collect<G, C, F> {
    pub fn new(generate: G, count: C) -> Self {
        let minimum = count.sample(0.0);
        Self {
            inner: generate,
            count,
            minimum,
            _marker: PhantomData,
        }
    }
}

impl<G: Generate, F: FromIterator<G::Item>> Generator<G, F> {
    pub fn new(generates: impl IntoIterator<Item = G>) -> Self {
        Self {
            inner: generates.into_iter().collect(),
            _marker: PhantomData,
        }
    }
}

impl<S: Shrink, F: FromIterator<S::Item>> Shrinker<S, F> {
    pub fn new(shrinks: Vec<S>, minimum: usize) -> Self {
        let maximum = shrinks.len();
        Self {
            inner: shrinks,
            index: 0,
            count: primitive::Shrinker::new(Range::usize(minimum..=maximum).unwrap(), maximum),
            minimum,
            _marker: PhantomData,
        }
    }

    pub fn shrinks(&self) -> &[S] {
        &self.inner
    }
}

impl<I: Clone, C: Clone, F> Clone for Collect<I, C, F> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            count: self.count.clone(),
            minimum: self.minimum,
            _marker: PhantomData,
        }
    }
}

impl<I: Clone, F> Clone for Shrinker<I, F> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            index: self.index,
            count: self.count.clone(),
            minimum: self.minimum,
            _marker: PhantomData,
        }
    }
}

impl<G: Generate + ?Sized, C: Generate<Item = usize>, F: FromIterator<G::Item>> Generate
    for Collect<G, C, F>
{
    type Item = F;
    type Shrink = Shrinker<G::Shrink, F>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let count = self.count.generate(state).item();
        let shrinks = Iterator::map(0..count, |_| self.inner.generate(state));
        Shrinker::new(shrinks.collect(), self.minimum)
    }

    fn constant(&self) -> bool {
        self.count.constant() && self.inner.constant()
    }
}

impl<G: Generate, F: FromIterator<G::Item> + Extend<G::Item> + Default> Generate
    for Generator<G, F>
{
    type Item = F;
    type Shrink = Shrinker<G::Shrink, F>;
    fn generate(&self, state: &mut State) -> Self::Shrink {
        let shrinks = self.inner.iter().map(|generate| generate.generate(state));
        Shrinker::new(shrinks.collect(), 0)
    }

    fn constant(&self) -> bool {
        self.inner.iter().all(G::constant)
    }
}

impl<S: Shrink, F: FromIterator<S::Item>> Shrink for Shrinker<S, F> {
    type Item = F;

    fn item(&self) -> Self::Item {
        self.inner.iter().map(S::item).collect()
    }

    fn shrink(&mut self) -> Option<Self> {
        // Try to truncate irrelevant generators aggressively.
        if let Some(count) = self.count.shrink() {
            let mut inner = self.inner.clone();
            inner.truncate(count.item());
            return Some(Self {
                inner,
                index: 0,
                count,
                minimum: self.minimum,
                _marker: PhantomData,
            });
        }

        // Try to remove irrelevant generators one by one.
        if self.index < self.inner.len() && self.minimum < self.inner.len() {
            let mut inner = self.inner.clone();
            inner.remove(self.index);
            self.index += 1;
            return Some(Self {
                inner,
                index: 0,
                count: self.count.clone(),
                minimum: self.minimum,
                _marker: PhantomData,
            });
        }

        // Try to shrink each generator and succeed if any generator is shrunk.
        let start = self.index;
        self.index += 1;
        for i in 0..self.inner.len() {
            let index = (start + i) % self.inner.len();
            if let Some(shrink) = self.inner[index].shrink() {
                let mut inner = self.inner.clone();
                inner[index] = shrink;
                return Some(Self {
                    inner,
                    index: self.index,
                    count: self.count.clone(),
                    minimum: self.minimum,
                    _marker: PhantomData,
                });
            }
        }

        None
    }
}

macro_rules! full {
    ($t:ty, $f:ty) => {
        impl<G: FullGenerate> FullGenerate for $t {
            type Item = $f;
            type Generate = Collect<G::Generate, Range<usize>, Self::Item>;
            fn generator() -> Self::Generate {
                G::generator().collect()
            }
        }
    };
}

macro_rules! into {
    ($t:ty, $g:ty, $f:ty) => {
        impl<G: IntoGenerate> IntoGenerate for $t {
            type Item = $f;
            type Generate = $g;
            fn generator(self) -> Self::Generate {
                self.into_iter().map(G::generator).collect()
            }
        }
    };
}

macro_rules! slice {
    ($t:ty, $f:ty) => {
        full!($t, $f);

        impl<G: Generate> Generate for $t {
            type Item = $f;
            type Shrink = Shrinker<G::Shrink, Self::Item>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                let shrinks = self
                    .iter()
                    .map(|generate| generate.generate(state))
                    .collect::<Vec<_>>();
                let minimum = shrinks.len();
                Shrinker::new(shrinks, minimum)
            }

            fn constant(&self) -> bool {
                self.iter().all(G::constant)
            }
        }
    };
}

macro_rules! collection {
    ($t:ty, $g:ty, $f:ty) => {
        full!($t, $f);
        into!($t, $g, $f);

        impl<G: Generate> Generate for $t {
            type Item = $f;
            type Shrink = Shrinker<G::Shrink, Self::Item>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                let shrinks = self
                    .iter()
                    .map(|generate| generate.generate(state))
                    .collect();
                Shrinker::new(shrinks, 0)
            }

            fn constant(&self) -> bool {
                self.iter().all(G::constant)
            }
        }
    };
}

collection!(Vec<G>, Vec<G::Generate>, Vec<G::Item>);
collection!(VecDeque<G>, VecDeque<G::Generate>, VecDeque<G::Item>);
collection!(LinkedList<G>, LinkedList<G::Generate>, LinkedList<G::Item>);
slice!([G], Box<[G::Item]>);
slice!(Box<[G]>, Box<[G::Item]>);
slice!(Rc<[G]>, Rc<[G::Item]>);
slice!(Arc<[G]>, Arc<[G::Item]>);

impl<G: IntoGenerate> IntoGenerate for Box<[G]> {
    type Item = Box<[G::Item]>;
    type Generate = Box<[G::Generate]>;
    fn generator(self) -> Self::Generate {
        self.into_vec().into_iter().map(G::generator).collect()
    }
}

impl FullGenerate for String {
    type Item = Self;
    type Generate = Collect<<char as FullGenerate>::Generate, Range<usize>, Self::Item>;
    fn generator() -> Self::Generate {
        char::generator().collect()
    }
}

impl Generate for String {
    type Item = Self;
    type Shrink = Shrinker<char, Self::Item>;
    fn generate(&self, _: &mut State) -> Self::Shrink {
        Shrinker::new(self.chars().collect(), 0)
    }
    fn constant(&self) -> bool {
        true
    }
}

impl<K: FullGenerate<Item = impl Ord>, V: FullGenerate> FullGenerate for BTreeMap<K, V> {
    type Item = BTreeMap<K::Item, V::Item>;
    type Generate = Collect<<(K, V) as FullGenerate>::Generate, Range<usize>, Self::Item>;
    fn generator() -> Self::Generate {
        <(K, V)>::generator().collect()
    }
}

impl<K: Ord + Clone, V: IntoGenerate> IntoGenerate for BTreeMap<K, V> {
    type Item = BTreeMap<K, V::Item>;
    type Generate = Generator<(Same<K>, V::Generate), Self::Item>;
    fn generator(self) -> Self::Generate {
        Generator::new(
            self.into_iter()
                .map(|(key, value)| (Same(key), value.generator())),
        )
    }
}

impl<K: Ord + Clone, V: Generate> Generate for BTreeMap<K, V> {
    type Item = BTreeMap<K, V::Item>;
    type Shrink = Shrinker<All<(Same<K>, V::Shrink)>, Self::Item>;
    fn generate(&self, state: &mut State) -> Self::Shrink {
        Generator::new(self.iter().map(|(key, value)| (Same(key.clone()), value))).generate(state)
    }
    fn constant(&self) -> bool {
        self.iter().all(|(_, value)| value.constant())
    }
}

impl<G: FullGenerate<Item = impl Ord>> FullGenerate for BTreeSet<G> {
    type Item = BTreeSet<G::Item>;
    type Generate = Collect<G::Generate, Range<usize>, Self::Item>;
    fn generator() -> Self::Generate {
        G::generator().collect()
    }
}

impl<G: IntoGenerate<Item = impl Ord>> IntoGenerate for BTreeSet<G> {
    type Item = BTreeSet<G::Item>;
    type Generate = Generator<G::Generate, Self::Item>;
    fn generator(self) -> Self::Generate {
        Generator::new(self.into_iter().map(G::generator))
    }
}

impl<G: Generate<Item = impl Ord>> Generate for BTreeSet<G> {
    type Item = BTreeSet<G::Item>;
    type Shrink = Shrinker<G::Shrink, Self::Item>;
    fn generate(&self, state: &mut State) -> Self::Shrink {
        Generator::new(self).generate(state)
    }
    fn constant(&self) -> bool {
        self.iter().all(G::constant)
    }
}

impl<K: FullGenerate<Item = impl Eq + Hash>, V: FullGenerate, S: BuildHasher + Default> FullGenerate
    for HashMap<K, V, S>
{
    type Item = HashMap<K::Item, V::Item, S>;
    type Generate = Collect<<(K, V) as FullGenerate>::Generate, Range<usize>, Self::Item>;
    fn generator() -> Self::Generate {
        <(K, V)>::generator().collect()
    }
}

impl<K: Eq + Hash + Clone, V: IntoGenerate, S: BuildHasher + Default> IntoGenerate
    for HashMap<K, V, S>
{
    type Item = HashMap<K, V::Item, S>;
    type Generate = Generator<(Same<K>, V::Generate), Self::Item>;
    fn generator(self) -> Self::Generate {
        Generator::new(
            self.into_iter()
                .map(|(key, value)| (Same(key), value.generator())),
        )
    }
}

impl<K: Eq + Hash + Clone, V: Generate, S: BuildHasher + Default> Generate for HashMap<K, V, S> {
    type Item = HashMap<K, V::Item, S>;
    type Shrink = Shrinker<All<(Same<K>, V::Shrink)>, Self::Item>;
    fn generate(&self, state: &mut State) -> Self::Shrink {
        Generator::new(self.iter().map(|(key, value)| (Same(key.clone()), value))).generate(state)
    }
    fn constant(&self) -> bool {
        self.iter().all(|(_, value)| value.constant())
    }
}

impl<G: FullGenerate<Item = impl Eq + Hash>, S: BuildHasher + Default> FullGenerate
    for HashSet<G, S>
{
    type Item = HashSet<G::Item, S>;
    type Generate = Collect<G::Generate, Range<usize>, Self::Item>;
    fn generator() -> Self::Generate {
        G::generator().collect()
    }
}

impl<G: IntoGenerate<Item = impl Eq + Hash>> IntoGenerate for HashSet<G> {
    type Item = HashSet<G::Item>;
    type Generate = Generator<G::Generate, Self::Item>;
    fn generator(self) -> Self::Generate {
        Generator::new(self.into_iter().map(G::generator))
    }
}

impl<G: Generate<Item = impl Eq + Hash>> Generate for HashSet<G> {
    type Item = HashSet<G::Item>;
    type Shrink = Shrinker<G::Shrink, Self::Item>;
    fn generate(&self, state: &mut State) -> Self::Shrink {
        Generator::new(self).generate(state)
    }
    fn constant(&self) -> bool {
        self.iter().all(G::constant)
    }
}

impl<G: FullGenerate<Item = impl Ord>> FullGenerate for BinaryHeap<G> {
    type Item = BinaryHeap<G::Item>;
    type Generate = Collect<G::Generate, Range<usize>, Self::Item>;
    fn generator() -> Self::Generate {
        G::generator().collect()
    }
}

impl<G: IntoGenerate<Item = impl Ord>> IntoGenerate for BinaryHeap<G> {
    type Item = BinaryHeap<G::Item>;
    type Generate = Generator<G::Generate, Self::Item>;
    fn generator(self) -> Self::Generate {
        Generator::new(self.into_iter().map(G::generator))
    }
}

impl<G: Generate<Item = impl Ord>> Generate for BinaryHeap<G> {
    type Item = BinaryHeap<G::Item>;
    type Shrink = Shrinker<G::Shrink, Self::Item>;
    fn generate(&self, state: &mut State) -> Self::Shrink {
        Generator::new(self).generate(state)
    }
    fn constant(&self) -> bool {
        self.iter().all(G::constant)
    }
}
