use crate::{
    generate::{self, Generator, State},
    shrink::Shrinker,
};

#[derive(Debug, Default, Clone)]
pub struct FilterMap<G: ?Sized, F> {
    map: F,
    retries: usize,
    generator: G,
}

#[derive(Debug, Clone)]
pub struct Shrink<S, F> {
    shrinker: Option<S>,
    map: F,
}

impl<G: Generator, T, F: Fn(G::Item) -> Option<T>> FilterMap<G, F> {
    pub const fn new(generator: G, map: F, retries: usize) -> Self {
        Self {
            generator,
            map,
            retries,
        }
    }
}

impl<G: Generator + ?Sized, T, F: Fn(G::Item) -> Option<T> + Clone> Generator for FilterMap<G, F> {
    type Item = Option<T>;
    type Shrink = Shrink<G::Shrink, F>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let mut outer = None;
        let size = state.size;
        for i in 0..self.retries {
            state.size = generate::size(i, self.retries, size.0..size.1);
            let inner = self.generator.generate(state);
            let item = inner.item();
            if self.constant() || (self.map)(item).is_some() {
                outer = Some(inner);
                break;
            }
        }
        state.size = size;
        Shrink {
            shrinker: outer,
            map: self.map.clone(),
        }
    }

    fn constant(&self) -> bool {
        self.retries == 0 || self.generator.constant()
    }
}

impl<S: Shrinker, T, F: Fn(S::Item) -> Option<T> + Clone> Shrinker for Shrink<S, F> {
    type Item = Option<T>;

    fn item(&self) -> Self::Item {
        (self.map)(self.shrinker.as_ref()?.item())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self {
            shrinker: Some(self.shrinker.as_mut()?.shrink()?),
            map: self.map.clone(),
        })
    }
}
