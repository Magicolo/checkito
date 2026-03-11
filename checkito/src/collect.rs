use crate::{
    COLLECTS, all, cardinality,
    generate::{FullGenerate, Generate},
    primitive::{self, Constant, Direction, Full, Range, usize::Usize},
    shrink::Shrink,
    state::State,
};
use core::{marker::PhantomData, mem::replace};
use std::{
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque},
    rc::Rc,
    sync::Arc,
};

#[derive(Debug)]
pub struct Collect<I: ?Sized, C, F: ?Sized> {
    pub(crate) _marker: PhantomData<F>,
    pub(crate) count: C,
    pub(crate) generator: I,
}

#[derive(Debug)]
pub struct Shrinker<S, F: ?Sized> {
    pub(crate) shrinkers: Vec<S>,
    pub(crate) machine: Machine,
    pub(crate) minimum: usize,
    _marker: PhantomData<F>,
}

pub trait Count {
    const COUNT: Option<Range<usize>> = None;
    fn count(&self) -> Range<usize>;
}

/// State machine for multi-phase collection shrinking.
///
/// Collection shrinking proceeds in phases to find the simplest failing case:
/// 1. `Truncate`: Reduce the collection size by removing elements from the end
/// 2. `Remove`: Try removing individual elements one at a time
/// 3. `Shrink`: Shrink individual elements in place
/// 4. `Done`: No more shrinking possible
///
/// This phased approach helps find minimal test cases by first trying
/// structural simplifications (fewer elements) before content simplifications
/// (simpler elements).
#[derive(Debug, Clone)]
pub(crate) enum Machine {
    Truncate(primitive::Shrinker<usize>),
    Remove(usize),
    Shrink(usize),
    Done,
}

type Default = Range<Usize<0>, Usize<COLLECTS>>;

impl<G: Generate, F: FromIterator<G::Item>> Collect<G, Default, F> {
    pub(crate) const fn new(generator: G) -> Self {
        Self {
            generator,
            count: Constant::VALUE,
            _marker: PhantomData,
        }
    }
}

impl<S: Shrink, F: FromIterator<S::Item>> Shrinker<S, F> {
    pub(crate) fn new(shrinkers: Vec<S>, minimum: usize) -> Self {
        let item = shrinkers.len();
        Self {
            shrinkers,
            machine: Machine::Truncate(primitive::Shrinker {
                start: minimum,
                end: item,
                item,
                direction: Direction::None,
            }),
            minimum,
            _marker: PhantomData,
        }
    }
}

impl<I: Clone, C: Clone, F> Clone for Collect<I, C, F> {
    fn clone(&self) -> Self {
        Self {
            generator: self.generator.clone(),
            count: self.count.clone(),
            _marker: PhantomData,
        }
    }
}

impl<I: Clone, F> Clone for Shrinker<I, F> {
    fn clone(&self) -> Self {
        Self {
            shrinkers: self.shrinkers.clone(),
            machine: self.machine.clone(),
            minimum: self.minimum,
            _marker: PhantomData,
        }
    }
}

impl<G: Generate + ?Sized, C: Count, F: FromIterator<G::Item>> Generate for Collect<G, C, F> {
    type Item = F;
    type Shrink = Shrinker<G::Shrink, F>;

    const CARDINALITY: Option<u128> = match C::COUNT {
        Some(count) => cardinality::all_repeat_dynamic(G::CARDINALITY, count),
        None => None,
    };

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let range = self.count.count();
        let shrinkers = state
            .repeat(&self.generator, range)
            .map(|generator| generator.generate(state))
            .collect();
        Shrinker::new(shrinkers, range.start())
    }

    fn cardinality(&self) -> Option<u128> {
        cardinality::all_repeat_dynamic(self.generator.cardinality(), self.count.count())
    }
}

impl<S: Shrink, F: FromIterator<S::Item>> Shrink for Shrinker<S, F> {
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
                Machine::Shrink(mut index) => match all::shrink(&mut self.shrinkers, &mut index) {
                    Some(shrinkers) => {
                        self.machine = Machine::Shrink(index);
                        break Some(Self {
                            shrinkers,
                            machine: Machine::Shrink(index),
                            minimum: self.minimum,
                            _marker: PhantomData,
                        });
                    }
                    None => self.machine = Machine::Done,
                },
                Machine::Done => break None,
            }
        }
    }
}

impl FullGenerate for String {
    type Generator = Collect<Full<char>, Default, Self::Item>;
    type Item = String;

    fn generator() -> Self::Generator {
        Collect::new(char::generator())
    }
}

macro_rules! slice {
    ($pointer: ident) => {
        impl<G: FullGenerate> FullGenerate for $pointer<[G]> {
            type Generator = Collect<G::Generator, Default, Self::Item>;
            type Item = $pointer<[G::Item]>;

            fn generator() -> Self::Generator {
                Collect::new(G::generator())
            }
        }
    };
}

macro_rules! list {
    ($type:ident $(, $bound:path)*) => {
        impl<G: FullGenerate> FullGenerate for $type<G>
        where
            $(G::Item: $bound,)*
        {
            type Generator = Collect<G::Generator, Default, Self::Item>;
            type Item = $type<G::Item>;

            fn generator() -> Self::Generator {
                Collect::new(G::generator())
            }
        }
    };
}

macro_rules! map {
    ($type:ident $(, $bound:path)*) => {
        impl<K: FullGenerate, V: FullGenerate> FullGenerate for $type<K, V>
        where
            $(K::Item: $bound,)*
        {
            type Generator = Collect<(K::Generator, V::Generator), Default, Self::Item>;
            type Item = $type<K::Item, V::Item>;

            fn generator() -> Self::Generator {
                Collect::new((K::generator(), V::generator()))
            }
        }
    };
}

slice!(Box);
slice!(Rc);
slice!(Arc);
list!(Vec);
list!(VecDeque);
list!(LinkedList);
list!(BinaryHeap, Ord);
list!(HashSet, Eq, core::hash::Hash);
list!(BTreeSet, Ord);
map!(HashMap, Eq, core::hash::Hash);
map!(BTreeMap, Ord);

impl<C: Count + ?Sized> Count for &C {
    const COUNT: Option<Range<usize>> = C::COUNT;

    fn count(&self) -> Range<usize> {
        C::count(self)
    }
}

impl<C: Count + ?Sized> Count for &mut C {
    const COUNT: Option<Range<usize>> = C::COUNT;

    fn count(&self) -> Range<usize> {
        C::count(self)
    }
}

impl<I: Constant, C: Constant, F> Constant for Collect<I, C, F> {
    const VALUE: Self = Self {
        _marker: PhantomData,
        count: C::VALUE,
        generator: I::VALUE,
    };
}
