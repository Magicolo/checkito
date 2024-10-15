use crate::{
    generate::{self, Generate, State},
    shrink::Shrink,
};

#[derive(Debug, Clone)]
pub struct FilterMap<G: ?Sized, F> {
    pub(crate) filter: F,
    pub(crate) retries: usize,
    pub(crate) generator: G,
}

#[derive(Debug, Clone)]
pub struct Shrinker<S, F> {
    shrinker: Option<S>,
    map: F,
}

impl<G: Generate + ?Sized, T, F: Fn(G::Item) -> Option<T> + Clone> Generate for FilterMap<G, F> {
    type Item = Option<T>;
    type Shrink = Shrinker<G::Shrink, F>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let mut outer = None;
        let size = state.size;
        for i in 0..self.retries {
            state.size = generate::size(i, self.retries, size);
            let inner = self.generator.generate(state);
            let item = inner.item();
            if self.constant() || (self.filter)(item).is_some() {
                outer = Some(inner);
                break;
            }
        }
        state.size = size;
        Shrinker {
            shrinker: outer,
            map: self.filter.clone(),
        }
    }

    fn constant(&self) -> bool {
        self.retries == 0 || self.generator.constant()
    }
}

impl<S: Shrink, T, F: Fn(S::Item) -> Option<T> + Clone> Shrink for Shrinker<S, F> {
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
