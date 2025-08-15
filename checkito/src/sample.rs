//! A utility for generating random values from a generator without running tests.
use crate::{
    generate::Generate,
    shrink::{Shrink, Shrinkers},
    state::{self, Modes, Sizes, State},
    SAMPLES,
};
use core::iter;

/// Configures the sampling process.
///
/// This struct is created by the [`Sample::sampler`] method and provides a
/// builder-like interface for configuring how values are sampled.
#[derive(Debug, Clone)]
pub struct Sampler<G: ?Sized> {
    /// The seed for the random number generator.
    ///
    /// Using the same seed will cause the sampler to produce the same sequence of
    /// random values. It defaults to a random value.
    pub seed: u64,
    /// The range of sizes (`0.0..=1.0`) that will be gradually traversed while
    /// generating values.
    ///
    /// Defaults to `0.0..=1.0`.
    pub sizes: Sizes,
    /// The number of samples to generate.
    ///
    /// Defaults to `SAMPLES`.
    pub count: usize,
    /// The generator that will provide the samples.
    pub generator: G,
}

/// An iterator that yields random values from a generator.
///
/// This struct is created by the [`Sample::samples`] method.
#[derive(Debug, Clone)]
pub struct Samples<G: ?Sized>(Shrinkers<G>);

/// An extension trait, implemented for all [`Generate`] types, that provides
/// methods for sampling random values.
pub trait Sample: Generate {
    /// Creates a [`Sampler`] for this generator.
    ///
    /// The `Sampler` can be used to configure and control the sampling process.
    fn sampler(self) -> Sampler<Self>
    where
        Self: Sized,
    {
        Sampler::new(self, state::seed())
    }

    /// Creates an iterator that generates `count` random values.
    ///
    /// The generated values will have progressively larger sizes. For more control
    /// over the sampling process, see [`Sample::sampler`].
    fn samples(self, count: usize) -> Samples<Self>
    where
        Self: Sized,
    {
        let mut sampler = self.sampler();
        sampler.count = count;
        sampler.samples()
    }

    /// Generates a single random value of a specific `size`.
    ///
    /// The `size` should be between `0.0` and `1.0`. For more control over the
    /// sampling process, see [`Sample::sampler`].
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
