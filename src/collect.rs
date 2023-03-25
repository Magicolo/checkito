use crate::{
    constant::Constant,
    generate::{FullGenerate, Generate, IntoGenerate, State},
    primitive::Range,
    shrink::Shrink,
};
use std::{
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque},
    hash::{BuildHasher, Hash},
    marker::PhantomData,
    rc::Rc,
    sync::Arc,
};

#[derive(Debug, Default)]
pub struct Collect<I: ?Sized, C, F: ?Sized> {
    _marker: PhantomData<F>,
    count: C,
    inner: I,
}

#[derive(Debug, Default)]
pub struct Generator<I, F: ?Sized> {
    inner: Vec<I>,
    _marker: PhantomData<F>,
}

#[derive(Debug, Default)]
pub struct Shrinker<I, F: ?Sized> {
    inner: Vec<I>,
    index: usize,
    _marker: PhantomData<F>,
}

impl<G: Generate, C: Generate<Item = usize>, F: FromIterator<G::Item>> Collect<G, C, F> {
    pub const fn new(generate: G, count: C) -> Self {
        Self {
            inner: generate,
            count,
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
    pub const fn new(shrinks: Vec<S>) -> Self {
        Self {
            inner: shrinks,
            index: 0,
            _marker: PhantomData,
        }
    }
}

impl<I: Clone, C: Clone, F> Clone for Collect<I, C, F> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            count: self.count.clone(),
            _marker: PhantomData,
        }
    }
}

impl<I: Clone, F> Clone for Shrinker<I, F> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            index: self.index,
            _marker: PhantomData,
        }
    }
}

impl<G: Generate + ?Sized, C: Generate<Item = usize>, F: FromIterator<G::Item>> Generate
    for Collect<G, C, F>
{
    type Item = F;
    type Shrink = Shrinker<G::Shrink, F>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let (count, _) = self.count.generate(state);
        let mut shrinks = Vec::with_capacity(count);
        let items = Iterator::map(0..count, |_| {
            let (item, state) = self.inner.generate(state);
            shrinks.push(state);
            item
        })
        .collect();
        (items, Shrinker::new(shrinks))
    }
}

impl<G: Generate, F: FromIterator<G::Item> + Extend<G::Item> + Default> Generate
    for Generator<G, F>
{
    type Item = F;
    type Shrink = Shrinker<G::Shrink, F>;
    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let (items, shrinks) = self
            .inner
            .iter()
            .map(|generate| generate.generate(state))
            .unzip();
        (items, Shrinker::new(shrinks))
    }
}

impl<S: Shrink, F: FromIterator<S::Item>> Shrink for Shrinker<S, F> {
    type Item = F;

    fn generate(&self) -> Self::Item {
        self.inner.iter().map(|shrink| shrink.generate()).collect()
    }

    fn shrink(&mut self) -> Option<Self> {
        // Try to remove irrelevant generators.
        if self.index < self.inner.len() {
            let mut shrinks = self.inner.clone();
            shrinks.remove(self.index);
            self.index += 1;
            return Some(Self::new(shrinks));
        }

        // Try to shrink each generator and succeed if any generator is shrunk.
        for i in 0..self.inner.len() {
            if let Some(shrink) = self.inner[i].shrink() {
                let mut shrinks = self.inner.clone();
                shrinks[i] = shrink;
                return Some(Self::new(shrinks));
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

            fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                let (items, shrinks): (Vec<_>, _) =
                    self.iter().map(|generate| generate.generate(state)).unzip();
                (items.into(), Shrinker::new(shrinks))
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

            fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                let (items, shrinks) = self.iter().map(|generate| generate.generate(state)).unzip();
                (items, Shrinker::new(shrinks))
            }
        }
    };
}

collection!(Vec<G>, Vec<G::Generate>, Vec<G::Item>);
collection!(VecDeque<G>, VecDeque<G::Generate>, VecDeque<G::Item>);
collection!(LinkedList<G>, LinkedList<G::Generate>, LinkedList<G::Item>);
slice!(Box<[G]>, Box<[G::Item]>);
slice!(Rc<[G]>, Rc<[G::Item]>);
slice!(Arc<[G]>, Arc<[G::Item]>);

impl<G: IntoGenerate> IntoGenerate for Box<[G]> {
    type Item = Box<[G::Item]>;
    type Generate = Box<[G::Generate]>;
    fn generator(self) -> Self::Generate {
        Vec::from(self).generator().into()
    }
}

impl FullGenerate for String {
    type Item = Self;
    type Generate = Collect<<char as FullGenerate>::Generate, Range<usize>, Self::Item>;
    fn generator() -> Self::Generate {
        char::generator().collect()
    }
}

impl IntoGenerate for String {
    type Item = Self;
    type Generate = Self;
    fn generator(self) -> Self::Generate {
        self
    }
}

impl Generate for String {
    type Item = Self;
    type Shrink = Shrinker<char, Self::Item>;
    fn generate(&self, _: &mut State) -> (Self::Item, Self::Shrink) {
        (self.clone(), Shrinker::new(self.chars().collect()))
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
    type Generate = Generator<(Constant<K>, V::Generate), Self::Item>;
    fn generator(self) -> Self::Generate {
        Generator::new(
            self.into_iter()
                .map(|(key, value)| (Constant(key), value.generator())),
        )
    }
}

impl<K: Ord + Clone, V: Generate> Generate for BTreeMap<K, V> {
    type Item = BTreeMap<K, V::Item>;
    type Shrink = Shrinker<(Constant<K>, V::Shrink), Self::Item>;
    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        Generator::new(
            self.into_iter()
                .map(|(key, value)| (Constant(key.clone()), value)),
        )
        .generate(state)
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
    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        Generator::new(self).generate(state)
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
    type Generate = Generator<(Constant<K>, V::Generate), Self::Item>;
    fn generator(self) -> Self::Generate {
        Generator::new(
            self.into_iter()
                .map(|(key, value)| (Constant(key), value.generator())),
        )
    }
}

impl<K: Eq + Hash + Clone, V: Generate, S: BuildHasher + Default> Generate for HashMap<K, V, S> {
    type Item = HashMap<K, V::Item, S>;
    type Shrink = Shrinker<(Constant<K>, V::Shrink), Self::Item>;
    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        Generator::new(
            self.into_iter()
                .map(|(key, value)| (Constant(key.clone()), value)),
        )
        .generate(state)
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
    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        Generator::new(self).generate(state)
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
    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        Generator::new(self).generate(state)
    }
}
