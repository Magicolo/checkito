use crate::{
    COLLECT, all,
    generate::{FullGenerate, Generate},
    primitive::{self, Direction, Full},
    shrink::Shrink,
    state::State,
};
use core::{marker::PhantomData, mem::replace, ops::RangeInclusive};

#[derive(Debug)]
pub struct Collect<I: ?Sized, C, F: ?Sized> {
    pub(crate) _marker: PhantomData<F>,
    pub(crate) count: C,
    pub(crate) minimum: Option<usize>,
    pub(crate) generator: I,
}

#[derive(Debug)]
pub struct Shrinker<S, F: ?Sized> {
    pub(crate) shrinkers: Vec<S>,
    pub(crate) machine: Machine,
    pub(crate) minimum: usize,
    _marker: PhantomData<F>,
}

#[derive(Debug, Clone)]
pub(crate) enum Machine {
    Truncate(primitive::Shrinker<usize>),
    Remove(usize),
    Shrink(usize),
    Done,
}

impl<G: Generate, F: FromIterator<G::Item>> Collect<G, RangeInclusive<usize>, F> {
    pub(crate) const fn new(generator: G) -> Self {
        Self {
            generator,
            count: 0..=COLLECT,
            minimum: Some(0),
            _marker: PhantomData,
        }
    }
}

impl<S: Shrink, F: FromIterator<S::Item>> Shrinker<S, F> {
    pub(crate) fn new(shrinkers: impl IntoIterator<Item = S>, minimum: Option<usize>) -> Self {
        let shrinkers = shrinkers.into_iter().collect::<Vec<_>>();
        let minimum = minimum.unwrap_or(shrinkers.len());
        let maximum = shrinkers.len();
        Self {
            shrinkers,
            machine: Machine::Truncate(primitive::Shrinker {
                start: minimum,
                end: maximum,
                item: maximum,
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
            minimum: self.minimum,
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

// TODO:
// trait Bound<T> {
//     const LOWER: T;
//     const UPPER: T;
//     fn lower(&self) -> T;
//     fn upper(&self) -> T;
// }

// struct Constant<const N: usize>;
// struct Range<const L: usize, const U: usize>;
// impl<const N: usize> Bound<usize> for Constant<N> {
//     const LOWER: usize = N;
//     const UPPER: usize = N;
// }
// impl<const L: usize, const U: usize> Bound<usize> for Range<L, U> {
//     const LOWER: usize = L;
//     const UPPER: usize = U;
// }
// impl Bound<usize> for usize {
//     const LOWER: usize = 0;
//     const UPPER: usize = usize::MAX;
// }

impl<G: Generate + ?Sized, C: Generate<Item = usize>, F: FromIterator<G::Item>> Generate
    for Collect<G, C, F>
{
    type Item = F;
    type Shrink = Shrinker<G::Shrink, F>;

    // TODO: If the largest value of the set `C::Item` can be known statically, then
    // the cardinality can be estimated.
    const CARDINALITY: Option<usize> = None;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let count = self.count.generate(state).item();
        let shrinkers = Iterator::map(0..count, |_| self.generator.generate(state));
        Shrinker::new(shrinkers, self.minimum)
    }

    fn cardinality(&self) -> Option<usize> {
        // TODO: If the largest value of the set `C::Item` can be known dynamically,
        // then the cardinality can be estimated. Otherwise, unless `C::cardinality() ==
        // 0`, the values in the set `C::Item` can not be known.
        None
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

impl<G: FullGenerate> FullGenerate for Vec<G> {
    type Generator = Collect<G::Generator, RangeInclusive<usize>, Self::Item>;
    type Item = Vec<G::Item>;

    fn generator() -> Self::Generator {
        Collect::new(G::generator())
    }
}

impl FullGenerate for String {
    type Generator = Collect<Full<char>, RangeInclusive<usize>, Self::Item>;
    type Item = String;

    fn generator() -> Self::Generator {
        Collect::new(char::generator())
    }
}
