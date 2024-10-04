use crate::{
    generate::{self, Generator, State},
    shrink::Shrinker,
};

#[derive(Clone, Debug, Default)]
pub struct Filter<G: ?Sized, F> {
    filter: F,
    retries: usize,
    generator: G,
}

#[derive(Clone, Debug, Default)]
pub struct Shrink<S, F> {
    shrinker: Option<S>,
    filter: F,
}

impl<G: Generator, F: Fn(&G::Item) -> bool> Filter<G, F> {
    pub const fn new(generator: G, filter: F, retries: usize) -> Self {
        Self {
            generator,
            filter,
            retries,
        }
    }
}

impl<G: Generator + ?Sized, F: Fn(&G::Item) -> bool + Clone> Generator for Filter<G, F> {
    type Item = Option<G::Item>;
    type Shrink = Shrink<G::Shrink, F>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let mut outer = None;
        let size = state.size;
        for i in 0..self.retries {
            state.size = generate::size(i, self.retries, size.0..size.1);
            let inner = self.generator.generate(state);
            let item = inner.item();
            if self.constant() || (self.filter)(&item) {
                outer = Some(inner);
                break;
            }
        }
        state.size = size;
        Shrink {
            shrinker: outer,
            filter: self.filter.clone(),
        }
    }

    fn constant(&self) -> bool {
        self.retries == 0 || self.generator.constant()
    }
}

impl<S: Shrinker, F: Fn(&S::Item) -> bool + Clone> Shrinker for Shrink<S, F> {
    type Item = Option<S::Item>;

    fn item(&self) -> Self::Item {
        self.shrinker.item().filter(&self.filter)
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Shrink {
            filter: self.filter.clone(),
            shrinker: self.shrinker.shrink()?,
        })
    }
}
