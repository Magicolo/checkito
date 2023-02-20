use crate::generate::{Generate, State};
use fastrand::Rng;

#[derive(Debug)]
pub struct Sample<'a, G: ?Sized> {
    generate: &'a G,
    index: usize,
    count: usize,
    random: Rng,
}

impl<'a, G: ?Sized> Sample<'a, G> {
    pub fn new(generate: &'a G, count: usize) -> Self {
        Self {
            generate,
            index: 0,
            count,
            random: Rng::new(),
        }
    }
}

impl<G: Generate + ?Sized> Iterator for Sample<'_, G> {
    type Item = G::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.count {
            let mut state = State::new(self.index, self.count, self.random.u64(..));
            self.index += 1;
            Some(self.generate.generate(&mut state).0)
        } else {
            None
        }
    }
}

impl<G: Generate + ?Sized> ExactSizeIterator for Sample<'_, G> {
    #[inline]
    fn len(&self) -> usize {
        self.count - self.index
    }
}
