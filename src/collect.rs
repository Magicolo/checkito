use crate::{
    all::All,
    generate::{FullGenerator, Generator, IntoGenerator, State},
    primitive::{self, Range},
    same::Same,
    sample::Sample,
    shrink::Shrinker,
};
use core::{
    hash::{BuildHasher, Hash},
    marker::PhantomData,
    mem::replace,
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
    generator: I,
}

#[derive(Debug, Default)]
pub struct Gen<G, F: ?Sized> {
    generators: Vec<G>,
    _marker: PhantomData<F>,
}

#[derive(Debug)]
pub struct Shrink<S, F: ?Sized> {
    shrinkers: Vec<S>,
    machine: Machine,
    minimum: usize,
    _marker: PhantomData<F>,
}

#[derive(Debug, Clone)]
enum Machine {
    Truncate(primitive::Shrink<usize>),
    Remove(usize),
    Shrink(usize),
    Done,
}

impl<G: Generator, C: Generator<Item = usize>, F: FromIterator<G::Item>> Collect<G, C, F> {
    pub fn new(generator: G, count: C) -> Self {
        let minimum = count.sample(0.0);
        Self {
            generator,
            count,
            minimum,
            _marker: PhantomData,
        }
    }
}

impl<G: Generator, F: FromIterator<G::Item>> Gen<G, F> {
    pub(crate) fn new(generators: impl IntoIterator<Item = G>) -> Self {
        Self {
            generators: generators.into_iter().collect(),
            _marker: PhantomData,
        }
    }
}

impl<S: Shrinker, F: FromIterator<S::Item>> Shrink<S, F> {
    pub(crate) fn new(shrinkers: impl IntoIterator<Item = S>, minimum: Option<usize>) -> Self {
        let shrinkers = shrinkers.into_iter().collect::<Vec<_>>();
        let minimum = minimum.unwrap_or(shrinkers.len());
        let maximum = shrinkers.len();
        Self {
            shrinkers,
            machine: Machine::Truncate(primitive::Shrink::new(
                Range::usize(minimum..=maximum).unwrap(),
                maximum,
            )),
            minimum,
            _marker: PhantomData,
        }
    }

    pub fn shrinkers(&self) -> &[S] {
        &self.shrinkers
    }
}

impl<I: Clone, C: Clone, F> Clone for Collect<I, C, F> {
    fn clone(&self) -> Self {
        Self {
            generator: self.generator.clone(),
            count: self.count.clone(),
            minimum: self.minimum,
            _marker: PhantomData,
        }
    }
}

impl<I: Clone, F> Clone for Shrink<I, F> {
    fn clone(&self) -> Self {
        Self {
            shrinkers: self.shrinkers.clone(),
            machine: self.machine.clone(),
            minimum: self.minimum,
            _marker: PhantomData,
        }
    }
}

impl<G: Generator + ?Sized, C: Generator<Item = usize>, F: FromIterator<G::Item>> Generator
    for Collect<G, C, F>
{
    type Item = F;
    type Shrink = Shrink<G::Shrink, F>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let count = self.count.generate(state).item();
        let shrinkers = Iterator::map(0..count, |_| self.generator.generate(state));
        Shrink::new(shrinkers, Some(self.minimum))
    }

    fn constant(&self) -> bool {
        self.count.constant() && self.generator.constant()
    }
}

impl<G: Generator, F: FromIterator<G::Item> + Extend<G::Item> + Default> Generator for Gen<G, F> {
    type Item = F;
    type Shrink = Shrink<G::Shrink, F>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let shrinkers = self
            .generators
            .iter()
            .map(|generator| generator.generate(state));
        Shrink::new(shrinkers, Some(0))
    }

    fn constant(&self) -> bool {
        self.generators.iter().all(G::constant)
    }
}

impl<S: Shrinker, F: FromIterator<S::Item>> Shrinker for Shrink<S, F> {
    type Item = F;

    fn item(&self) -> Self::Item {
        self.shrinkers.iter().map(S::item).collect()
    }

    fn shrink(&mut self) -> Option<Self> {
        loop {
            match replace(&mut self.machine, Machine::Done) {
                // Try to truncate irrelevant generators aggressively.
                Machine::Truncate(mut outer) => match outer.shrink() {
                    Some(inner) => {
                        let mut shrinkers = self.shrinkers.clone();
                        shrinkers.truncate(inner.item());
                        self.machine = Machine::Truncate(outer);
                        break Some(Self {
                            shrinkers,
                            machine: Machine::Truncate(inner),
                            minimum: self.minimum,
                            _marker: PhantomData,
                        });
                    }
                    None => self.machine = Machine::Remove(0),
                },
                // Try to remove irrelevant generators one by one.
                Machine::Remove(index) => {
                    if index < self.shrinkers.len() && self.minimum < self.shrinkers.len() {
                        let mut shrinkers = self.shrinkers.clone();
                        shrinkers.remove(index);
                        self.machine = Machine::Remove(index + 1);
                        break Some(Self {
                            shrinkers,
                            machine: Machine::Remove(index),
                            minimum: self.minimum,
                            _marker: PhantomData,
                        });
                    } else {
                        self.machine = Machine::Shrink(0);
                    }
                }
                // Try to shrink each generator and succeed if any generator is shrunk.
                Machine::Shrink(index) => {
                    if let Some(old) = self.shrinkers.get_mut(index) {
                        if let Some(new) = old.shrink() {
                            let mut shrinkers = self.shrinkers.clone();
                            shrinkers[index] = new;
                            self.machine = Machine::Shrink(index);
                            return Some(Self {
                                shrinkers,
                                machine: Machine::Shrink(index),
                                minimum: self.minimum,
                                _marker: PhantomData,
                            });
                        } else {
                            self.machine = Machine::Shrink(index + 1);
                        }
                    } else {
                        self.machine = Machine::Done;
                    }
                }
                Machine::Done => break None,
            }
        }
    }
}

macro_rules! full {
    ($t:ty, $f:ty) => {
        impl<G: FullGenerator> FullGenerator for $t {
            type FullGen = Collect<G::FullGen, Range<usize>, Self::Item>;
            type Item = $f;

            fn full_gen() -> Self::FullGen {
                G::full_gen().collect()
            }
        }
    };
}

macro_rules! into {
    ($t:ty, $g:ty, $f:ty) => {
        impl<G: IntoGenerator> IntoGenerator for $t {
            type IntoGen = $g;
            type Item = $f;

            fn into_gen(self) -> Self::IntoGen {
                self.into_iter().map(G::into_gen).collect()
            }
        }
    };
}

macro_rules! slice {
    ($t:ty, $f:ty) => {
        full!($t, $f);

        impl<G: Generator> Generator for $t {
            type Item = $f;
            type Shrink = Shrink<G::Shrink, Self::Item>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Shrink::new(self.iter().map(|generator| generator.generate(state)), None)
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

        impl<G: Generator> Generator for $t {
            type Item = $f;
            type Shrink = Shrink<G::Shrink, Self::Item>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Shrink::new(
                    self.iter().map(|generator| generator.generate(state)),
                    Some(0),
                )
            }

            fn constant(&self) -> bool {
                self.iter().all(G::constant)
            }
        }
    };
}

collection!(Vec<G>, Vec<G::IntoGen>, Vec<G::Item>);
collection!(VecDeque<G>, VecDeque<G::IntoGen>, VecDeque<G::Item>);
collection!(LinkedList<G>, LinkedList<G::IntoGen>, LinkedList<G::Item>);
slice!([G], Box<[G::Item]>);
slice!(Box<[G]>, Box<[G::Item]>);
slice!(Rc<[G]>, Rc<[G::Item]>);
slice!(Arc<[G]>, Arc<[G::Item]>);

impl<G: IntoGenerator> IntoGenerator for Box<[G]> {
    type IntoGen = Box<[G::IntoGen]>;
    type Item = Box<[G::Item]>;

    fn into_gen(self) -> Self::IntoGen {
        self.into_vec().into_iter().map(G::into_gen).collect()
    }
}

impl FullGenerator for String {
    type FullGen = Collect<<char as FullGenerator>::FullGen, Range<usize>, Self::Item>;
    type Item = Self;

    fn full_gen() -> Self::FullGen {
        char::full_gen().collect()
    }
}

impl IntoGenerator for String {
    type IntoGen = Gen<char, Self::Item>;
    type Item = Self;

    fn into_gen(self) -> Self::IntoGen {
        Gen::new(self.chars())
    }
}

impl<K: FullGenerator<Item = impl Ord>, V: FullGenerator> FullGenerator for BTreeMap<K, V> {
    type FullGen = Collect<<(K, V) as FullGenerator>::FullGen, Range<usize>, Self::Item>;
    type Item = BTreeMap<K::Item, V::Item>;

    fn full_gen() -> Self::FullGen {
        <(K, V)>::full_gen().collect()
    }
}

impl<K: Ord + Clone, V: IntoGenerator> IntoGenerator for BTreeMap<K, V> {
    type IntoGen = Gen<All<(Same<K>, V::IntoGen)>, Self::Item>;
    type Item = BTreeMap<K, V::Item>;

    fn into_gen(self) -> Self::IntoGen {
        Gen::new(
            self.into_iter()
                .map(|(key, value)| All((Same(key), value.into_gen()))),
        )
    }
}

impl<G: FullGenerator<Item = impl Ord>> FullGenerator for BTreeSet<G> {
    type FullGen = Collect<G::FullGen, Range<usize>, Self::Item>;
    type Item = BTreeSet<G::Item>;

    fn full_gen() -> Self::FullGen {
        G::full_gen().collect()
    }
}

impl<G: IntoGenerator<Item = impl Ord>> IntoGenerator for BTreeSet<G> {
    type IntoGen = Gen<G::IntoGen, Self::Item>;
    type Item = BTreeSet<G::Item>;

    fn into_gen(self) -> Self::IntoGen {
        Gen::new(self.into_iter().map(G::into_gen))
    }
}

impl<K: FullGenerator<Item = impl Eq + Hash>, V: FullGenerator, S: BuildHasher + Default>
    FullGenerator for HashMap<K, V, S>
{
    type FullGen = Collect<<(K, V) as FullGenerator>::FullGen, Range<usize>, Self::Item>;
    type Item = HashMap<K::Item, V::Item, S>;

    fn full_gen() -> Self::FullGen {
        <(K, V)>::full_gen().collect()
    }
}

impl<K: Eq + Hash + Clone, V: IntoGenerator, S: BuildHasher + Default> IntoGenerator
    for HashMap<K, V, S>
{
    type IntoGen = Gen<All<(Same<K>, V::IntoGen)>, Self::Item>;
    type Item = HashMap<K, V::Item, S>;

    fn into_gen(self) -> Self::IntoGen {
        Gen::new(
            self.into_iter()
                .map(|(key, value)| All((Same(key), value.into_gen()))),
        )
    }
}

impl<G: FullGenerator<Item = impl Eq + Hash>, S: BuildHasher + Default> FullGenerator
    for HashSet<G, S>
{
    type FullGen = Collect<G::FullGen, Range<usize>, Self::Item>;
    type Item = HashSet<G::Item, S>;

    fn full_gen() -> Self::FullGen {
        G::full_gen().collect()
    }
}

impl<G: IntoGenerator<Item = impl Eq + Hash>> IntoGenerator for HashSet<G> {
    type IntoGen = Gen<G::IntoGen, Self::Item>;
    type Item = HashSet<G::Item>;

    fn into_gen(self) -> Self::IntoGen {
        Gen::new(self.into_iter().map(G::into_gen))
    }
}

impl<G: FullGenerator<Item = impl Ord>> FullGenerator for BinaryHeap<G> {
    type FullGen = Collect<G::FullGen, Range<usize>, Self::Item>;
    type Item = BinaryHeap<G::Item>;

    fn full_gen() -> Self::FullGen {
        G::full_gen().collect()
    }
}

impl<G: IntoGenerator<Item = impl Ord>> IntoGenerator for BinaryHeap<G> {
    type IntoGen = Gen<G::IntoGen, Self::Item>;
    type Item = BinaryHeap<G::Item>;

    fn into_gen(self) -> Self::IntoGen {
        Gen::new(self.into_iter().map(G::into_gen))
    }
}
