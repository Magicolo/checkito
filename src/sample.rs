use crate::{
    SAMPLES,
    generate::Generate,
    shrink::{Shrink, Shrinkers},
    state::{self, Modes, Sizes, State},
};
use core::iter;

#[derive(Debug, Clone)]
pub struct Sampler<G: ?Sized> {
    /// Seed for the random number generator used to generate random primitives.
    /// Defaults to a random value.
    pub seed: u64,
    /// Range of sizes that will be gradually traversed while generating values.
    /// Defaults to `0.0..1.0`.
    pub sizes: Sizes,
    /// Number of samples that will be generated.
    /// Defaults to `100`.
    pub count: usize,
    /// A generator that will provide the samples.
    pub generator: G,
}

#[derive(Debug, Clone)]
pub struct Samples<G: ?Sized>(Shrinkers<G>);

pub trait Sample: Generate {
    /// Provides a [`Sampler`] that allows to configure sampling settings and
    /// generate samples.
    fn sampler(self) -> Sampler<Self>
    where
        Self: Sized,
    {
        Sampler::new(self, state::seed())
    }

    /// Generates `count` random values that are progressively larger in size.
    /// For additional sampling settings, see [`Sample::sampler`].
    fn samples(self, count: usize) -> Samples<Self>
    where
        Self: Sized,
    {
        let mut sampler = self.sampler();
        sampler.count = count;
        sampler.samples()
    }

    /// Generates a random value of `size` (0.0..=1.0). For additional sampling
    /// settings, see [`Sample::sampler`].
    fn sample(&self, size: f64) -> Self::Item {
        self.sampler().sample(size)
    }
}

impl<G: Generate + ?Sized> Sample for G {}

impl<G> Sampler<G> {
    pub(crate) const fn new(generator: G, seed: u64) -> Self {
        Self {
            generator,
            seed,
            sizes: Sizes::DEFAULT,
            count: SAMPLES,
        }
    }
}

impl<G: Generate + ?Sized> Sampler<G> {
    pub fn sample(&self, size: f64) -> G::Item {
        let mut state = State::random(0, 1, size.into(), self.seed);
        self.generator.generate(&mut state).item()
    }
}

impl<G: Generate> Sampler<G> {
    pub fn samples(self) -> Samples<G> {
        let cardinality = self.generator.cardinality();
        Samples::new(
            self.generator,
            Modes::with(self.count, self.sizes, self.seed, cardinality, Some(false)),
        )
    }
}

impl<G: Generate> Samples<G> {
    pub(crate) fn new(generator: G, modes: Modes) -> Self {
        Self(Shrinkers::new(generator, modes))
    }
}

impl<G: Generate> Iterator for Samples<G> {
    type Item = G::Item;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.0.next()?.item())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    fn count(self) -> usize {
        self.0.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        Some(self.0.nth(n)?.item())
    }

    fn last(self) -> Option<Self::Item> {
        Some(self.0.last()?.item())
    }
}

impl<G: Generate> DoubleEndedIterator for Samples<G> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some(self.0.next_back()?.item())
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        Some(self.0.nth_back(n)?.item())
    }
}

impl<G: Generate> ExactSizeIterator for Samples<G> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<G: Generate> iter::FusedIterator for Samples<G> {}
