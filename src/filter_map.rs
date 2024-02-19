use crate::{
    generate::{Generate, State},
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
        let old = state.size;
        for i in 0..self.retries {
            let new = old + (1.0 - old) * (i as f64 / self.retries as f64);
            state.size = new.min(1.0).max(0.0);
            let inner = self.inner.generate(state);
            let item = inner.item();
            if (self.map)(item).is_some() {
                outer = Some(inner);
                break;
            }
        }
        state.size = old;
        Shrinker {
            inner: outer,
            map: self.map.clone(),
        }
    }
}

impl<S: Shrink + ?Sized, T, F: Fn(S::Item) -> Option<T> + Clone> Shrink for Shrinker<S, F> {
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
