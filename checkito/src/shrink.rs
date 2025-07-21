use crate::{
    generate::Generate,
    state::{Modes, State, States},
};
use core::iter;

pub trait Shrink: Clone {
    type Item;
    fn item(&self) -> Self::Item;
    fn shrink(&mut self) -> Option<Self>;
}

#[derive(Debug, Clone)]
pub struct Shrinker<T: ?Sized>(pub(crate) T);

#[derive(Debug, Clone)]
pub(crate) struct Shrinkers<G: ?Sized> {
    states: States,
    generator: G,
}

impl<G: Generate + ?Sized> Generate for Shrinker<G> {
    type Item = G::Shrink;
    type Shrink = Shrinker<G::Shrink>;

    const CARDINALITY: Option<u128> = G::CARDINALITY;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Shrinker(self.0.generate(state))
    }

    fn cardinality(&self) -> Option<u128> {
        self.0.cardinality()
    }
}

impl<S: Shrink> Shrink for Shrinker<S> {
    type Item = S;

    fn item(&self) -> Self::Item {
        self.0.clone()
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self(self.0.shrink()?))
    }
}

impl<G: Generate> Shrinkers<G> {
    pub(crate) fn new(generator: G, modes: Modes) -> Self {
        Shrinkers {
            generator,
            states: modes.into(),
        }
    }
}

impl<G: Generate + ?Sized> Iterator for Shrinkers<G> {
    type Item = G::Shrink;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.next()?))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.states.size_hint()
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.states.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.nth(n)?))
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        Some(self.generator.generate(&mut self.states.last()?))
    }
}

impl<G: Generate + ?Sized> DoubleEndedIterator for Shrinkers<G> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.next_back()?))
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.nth_back(n)?))
    }
}

impl<G: Generate + ?Sized> ExactSizeIterator for Shrinkers<G> {
    fn len(&self) -> usize {
        self.states.len()
    }
}

impl<G: Generate + ?Sized> iter::FusedIterator for Shrinkers<G> {}
