use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};
use fastrand::Rng;

#[derive(Debug)]
pub struct Sampler<'a, G: ?Sized> {
    pub generate: &'a G,
    /// Seed for the random number generator used to generate random primitives.
    pub seed: Option<u64>,
}

#[derive(Debug)]
pub struct Samples<'a, G: ?Sized> {
    sampler: Sampler<'a, G>,
    index: usize,
    count: usize,
    random: Rng,
}

impl<'a, G: ?Sized> Sampler<'a, G> {
    pub fn new(generate: &'a G, seed: Option<u64>) -> Self {
        Self { generate, seed }
    }
}

impl<G: ?Sized> Clone for Sampler<'_, G> {
    fn clone(&self) -> Self {
        Self {
            generate: self.generate,
            seed: self.seed,
        }
    }
}

impl<'a, G: Generate + ?Sized> Sampler<'a, G> {
    pub fn sample(&self, size: f64) -> G::Item {
        let mut state = State::new(size, self.seed);
        self.generate.generate(&mut state).item()
    }

    pub fn samples(&self, count: usize) -> Samples<'a, G> {
        Samples {
            sampler: self.clone(),
            index: 0,
            count,
            random: self.seed.map_or_else(Rng::new, Rng::with_seed),
        }
    }
}

impl<G: Generate + ?Sized> Iterator for Samples<'_, G> {
    type Item = G::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.count {
            let mut state =
                State::from_iteration(self.index, self.count, Some(self.random.u64(..)));
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
