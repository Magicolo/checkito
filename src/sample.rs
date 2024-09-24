use crate::{
    generate::{Generate, State},
    random::Random,
    shrink::Shrink,
};
use core::ops::Range;

#[derive(Debug)]
pub struct Sampler<'a, G: ?Sized> {
    /// A generator that will provide the samples.
    pub generate: &'a G,
    /// Seed for the random number generator used to generate random primitives.
    /// Defaults to a random value.
    pub seed: Option<u64>,
    /// Range of sizes that will be gradually traversed while generating values.
    /// Defaults to `0.0..1.0`.
    pub size: Range<f64>,
    /// Number of samples that will be generated.
    /// Defaults to `100`.
    pub count: usize,
}

#[derive(Debug)]
pub struct Samples<'a, G: ?Sized> {
    sampler: Sampler<'a, G>,
    index: usize,
    count: usize,
    random: Random,
}

const COUNT: usize = 100;

impl<'a, G: ?Sized> Sampler<'a, G> {
    pub fn new(generate: &'a G) -> Self {
        Self {
            generate,
            seed: None,
            size: 0.0..1.0,
            count: COUNT,
        }
    }
}

impl<G: ?Sized> Clone for Sampler<'_, G> {
    fn clone(&self) -> Self {
        Self {
            generate: self.generate,
            seed: self.seed,
            size: self.size.clone(),
            count: self.count,
        }
    }
}

impl<'a, G: Generate + ?Sized> Sampler<'a, G> {
    pub fn sample(&self, size: f64) -> G::Item {
        let mut state = State::new(size, self.seed);
        self.generate.generate(&mut state).item()
    }

    pub fn samples(&self) -> Samples<'a, G> {
        Samples {
            sampler: self.clone(),
            index: 0,
            count: self.count,
            random: Random::new(self.seed),
        }
    }
}

impl<G: Generate + ?Sized> Iterator for Samples<'_, G> {
    type Item = G::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.count {
            let mut state = State::from_iteration(
                self.index,
                self.count,
                self.sampler.size.clone(),
                Some(self.random.u64(..)),
            );
            self.index += 1;
            Some(self.sampler.generate.generate(&mut state).item())
        } else {
            None
        }
    }
}

impl<G: Generate + ?Sized> ExactSizeIterator for Samples<'_, G> {
    fn len(&self) -> usize {
        self.count - self.index
    }
}
