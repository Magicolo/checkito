use crate::{
    generate::{self, Generate, State},
    shrink::Shrink,
};

#[derive(Clone, Debug)]
pub struct Filter<G: ?Sized, F> {
    filter: F,
    retries: usize,
    generator: G,
}

#[derive(Clone, Debug)]
pub struct Shrinker<S, F> {
    shrinker: Option<S>,
    filter: F,
}

impl<G: Generate, F: Fn(&G::Item) -> bool> Filter<G, F> {
    pub const fn new(generator: G, filter: F, retries: usize) -> Self {
        Self {
            generator,
            filter,
            retries,
        }
    }
}

impl<G: Generate + ?Sized, F: Fn(&G::Item) -> bool + Clone> Generate for Filter<G, F> {
    type Item = Option<G::Item>;
    type Shrink = Shrinker<G::Shrink, F>;

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
        Shrinker {
            shrinker: outer,
            filter: self.filter.clone(),
        }
    }

    fn constant(&self) -> bool {
        self.retries == 0 || self.generator.constant()
    }
}

impl<S: Shrink, F: Fn(&S::Item) -> bool + Clone> Shrink for Shrinker<S, F> {
    type Item = Option<S::Item>;

    fn item(&self) -> Self::Item {
        let item = self.shrinker.as_ref()?.item();
        if (self.filter)(&item) {
            Some(item)
        } else {
            None
        }
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Shrinker {
            filter: self.filter.clone(),
            shrinker: Some(self.shrinker.as_mut()?.shrink()?),
        })
    }
}
