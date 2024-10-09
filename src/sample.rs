use crate::{
    generate::Generator,
    random,
    shrink::{Shrinker, Shrinkers, shrinker},
};
use core::{iter, ops::Range};

#[derive(Debug)]
pub struct Sampler<'a, G: ?Sized> {
    /// A generator that will provide the samples.
    pub generator: &'a G,
    /// Seed for the random number generator used to generate random primitives.
    /// Defaults to a random value.
    pub seed: u64,
    /// Range of sizes that will be gradually traversed while generating values.
    /// Defaults to `0.0..1.0`.
    pub size: Range<f64>,
    /// Number of samples that will be generated.
    /// Defaults to `100`.
    pub count: usize,
}

#[derive(Debug)]
pub struct Samples<'a, G: ?Sized>(Shrinkers<'a, G>);

pub trait Sample: Generator {
    /// Provides a [`Sampler`] that allows to configure sampling settings and
    /// generate samples.
    fn sampler(&self) -> Sampler<Self> {
        Sampler::new(self, random::seed())
    }

    /// Generates `count` random values the are progressively larger in size.
    /// For additional sampling settings, see [`Sample::sampler`].
    fn samples(&self, count: usize) -> Samples<Self> {
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

const COUNT: usize = 100;

impl<G: Generator + ?Sized> Sample for G {}

impl<'a, G: ?Sized> Sampler<'a, G> {
    pub(crate) const fn new(generator: &'a G, seed: u64) -> Self {
        Self {
            generator,
            seed,
            size: 0.0..1.0,
            count: COUNT,
        }
    }
}

impl<G: ?Sized> Clone for Sampler<'_, G> {
    fn clone(&self) -> Self {
        Self {
            generator: self.generator,
            seed: self.seed,
            size: self.size.clone(),
            count: self.count,
        }
    }
}

impl<'a, G: Generator + ?Sized> Sampler<'a, G> {
    pub fn sample(&self, size: f64) -> G::Item {
        shrinker(self.generator, size, Some(self.seed)).item()
    }

    pub fn samples(&self) -> Samples<'a, G> {
        Samples(Shrinkers::new(
            self.generator,
            self.count,
            self.size.clone(),
            Some(self.seed),
        ))
    }
}

impl<'a, G: Generator + ?Sized> From<&'a G> for Samples<'a, G> {
    fn from(value: &'a G) -> Self {
        Samples(Shrinkers::from(value))
    }
}

impl<G: Generator + ?Sized> Clone for Samples<'_, G> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<G: Generator + ?Sized> Iterator for Samples<'_, G> {
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

impl<G: Generator + ?Sized> DoubleEndedIterator for Samples<'_, G> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some(self.0.next_back()?.item())
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        Some(self.0.nth_back(n)?.item())
    }
}

impl<G: Generator + ?Sized> ExactSizeIterator for Samples<'_, G> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<G: Generator + ?Sized> iter::FusedIterator for Samples<'_, G> {}
