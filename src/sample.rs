use crate::generate::{Generate, State};
use fastrand::Rng;

#[derive(Debug)]
pub struct Sampler<'a, G: ?Sized> {
    pub generate: &'a G,
    pub count: usize,
    pub seed: Option<u64>,
}

#[derive(Debug)]
pub struct Samples<'a, G: ?Sized> {
    generate: &'a G,
    index: usize,
    count: usize,
    random: Rng,
}

impl<'a, G: ?Sized> Sampler<'a, G> {
    pub fn new(generate: &'a G, count: usize, seed: Option<u64>) -> Self {
        Self {
            generate,
            count,
            seed,
        }
    }
}

impl<'a, G: Generate + ?Sized> Sampler<'a, G> {
    pub fn sample(&self, size: f64) -> G::Item {
        let mut state = State::new(size.min(0.0).max(1.0), self.seed);
        self.generate.generate(&mut state).0
    }
}

impl<'a, G: Generate + ?Sized> IntoIterator for &Sampler<'a, G> {
    type Item = G::Item;
    type IntoIter = Samples<'a, G>;

    fn into_iter(self) -> Self::IntoIter {
        Samples {
            generate: self.generate,
            index: 0,
            count: self.count,
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
            Some(self.generate.generate(&mut state).0)
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
