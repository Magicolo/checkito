use crate::{
    generate::{self, Generate, State},
    shrink::Shrink,
};

#[derive(Debug, Default, Clone)]
pub struct FilterMap<G: ?Sized, F> {
    map: F,
    retries: usize,
    generator: G,
}

#[derive(Debug, Clone)]
pub struct Shrinkz<S, F> {
    shrinker: Option<S>,
    map: F,
}

impl<G: Generate, T, F: Fn(G::Item) -> Option<T>> FilterMap<G, F> {
    pub const fn new(generator: G, map: F, retries: usize) -> Self {
        Self {
            generator,
            map,
            retries,
        }
    }
}

impl<G: Generate + ?Sized, T, F: Fn(G::Item) -> Option<T> + Clone> Generate for FilterMap<G, F> {
    type Item = Option<T>;
    type Shrink = Shrinkz<G::Shrink, F>;

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
        Shrinkz {
            shrinker: outer,
            map: self.map.clone(),
        }
    }

    fn constant(&self) -> bool {
        self.retries == 0 || self.generator.constant()
    }
}

impl<S: Shrink, T, F: Fn(S::Item) -> Option<T> + Clone> Shrink for Shrinkz<S, F> {
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
