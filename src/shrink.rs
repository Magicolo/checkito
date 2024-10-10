use crate::{
    Generate, boxed, check,
    generate::{State, States},
    random,
};
use core::{iter, ops};

pub trait Shrink: Clone {
    type Item;

    fn item(&self) -> Self::Item;
    fn shrink(&mut self) -> Option<Self>;

    fn boxed(self) -> boxed::Shrinkz<Self::Item>
    where
        Self: 'static,
    {
        boxed::Shrinkz::new(self)
    }
}

#[derive(Debug)]
pub struct Shrinkers<'a, G: ?Sized> {
    generator: &'a G,
    states: States,
}

#[derive(Debug, Clone)]
pub struct Shrinker<T: ?Sized>(pub(crate) T);

impl<G: Generate + ?Sized> Generate for Shrinker<G> {
    type Item = G::Shrink;
    type Shrink = Shrinker<G::Shrink>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Shrinker(self.0.generate(state))
    }

    fn constant(&self) -> bool {
        self.0.constant()
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

impl<G: Generate + ?Sized> Clone for Shrinkers<'_, G> {
    fn clone(&self) -> Self {
        Self {
            generator: self.generator,
            states: self.states.clone(),
        }
    }
}

impl<'a, G: Generate + ?Sized> From<&'a G> for Shrinkers<'a, G> {
    fn from(value: &'a G) -> Self {
        Shrinkers::new(value, check::COUNT, 0.0..1.0, None)
    }
}

impl<'a, G: Generate + ?Sized> Shrinkers<'a, G> {
    pub fn new(generator: &'a G, count: usize, size: ops::Range<f64>, seed: Option<u64>) -> Self {
        Shrinkers {
            generator,
            states: States::new(count, size, seed),
        }
    }
}

pub(crate) fn shrinker<G: Generate + ?Sized>(
    generator: &G,
    size: f64,
    seed: Option<u64>,
) -> G::Shrink {
    let mut state = State::new(0, 1, size..size, seed.unwrap_or_else(random::seed));
    generator.generate(&mut state)
}

impl<G: Generate + ?Sized> Iterator for Shrinkers<'_, G> {
    type Item = G::Shrink;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.next()?))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.states.size_hint()
    }

    fn count(self) -> usize {
        self.states.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.nth(n)?))
    }

    fn last(self) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.last()?))
    }
}

impl<G: Generate + ?Sized> DoubleEndedIterator for Shrinkers<'_, G> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.next_back()?))
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.nth_back(n)?))
    }
}

impl<G: Generate + ?Sized> ExactSizeIterator for Shrinkers<'_, G> {
    fn len(&self) -> usize {
        self.states.len()
    }
}

impl<G: Generate + ?Sized> iter::FusedIterator for Shrinkers<'_, G> {}
