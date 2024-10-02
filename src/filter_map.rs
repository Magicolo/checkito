use crate::{
    generate::{self, Generate, State},
    shrink::Shrink,
};

#[derive(Debug, Default, Clone)]
pub struct FilterMap<I: ?Sized, F> {
    map: F,
    retries: usize,
    inner: I,
}

#[derive(Debug, Clone)]
pub struct Shrinker<I, F> {
    inner: Option<I>,
    map: F,
}

impl<G: Generate, T, F: Fn(G::Item) -> Option<T>> FilterMap<G, F> {
    pub const fn new(generate: G, map: F, retries: usize) -> Self {
        Self {
            inner: generate,
            map,
            retries,
        }
    }
}

impl<G: Generate + ?Sized, T, F: Fn(G::Item) -> Option<T> + Clone> Generate for FilterMap<G, F> {
    type Item = Option<T>;
    type Shrink = Shrinker<G::Shrink, F>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let mut outer = None;
        let size = state.size;
        for i in 0..self.retries {
            state.size = generate::size(i, self.retries, size.0..size.1);
            let inner = self.inner.generate(state);
            let item = inner.item();
            if self.constant() || (self.map)(item).is_some() {
                outer = Some(inner);
                break;
            }
        }
        state.size = size;
        Shrinker {
            inner: outer,
            map: self.map.clone(),
        }
    }

    fn constant(&self) -> bool {
        self.inner.constant()
    }
}

impl<S: Shrink, T, F: Fn(S::Item) -> Option<T> + Clone> Shrink for Shrinker<S, F> {
    type Item = Option<T>;

    fn item(&self) -> Self::Item {
        self.inner.item().and_then(&self.map)
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self {
            inner: self.inner.shrink()?,
            map: self.map.clone(),
        })
    }
}
