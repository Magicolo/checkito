use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};

#[derive(Debug, Default, Clone)]
pub struct Map<I: ?Sized, F> {
    map: F,
    inner: I,
}

#[derive(Debug, Clone)]
pub struct Shrinker<I: ?Sized, F> {
    map: F,
    inner: I,
}

impl<G: Generate, T, F: Fn(G::Item) -> T> Map<G, F> {
    pub const fn new(generate: G, map: F) -> Self {
        Self {
            inner: generate,
            map,
        }
    }
}

impl<S: Shrink, T, F: Fn(S::Item) -> T> Shrinker<S, F> {
    pub const fn new(shrink: S, map: F) -> Self {
        Self { inner: shrink, map }
    }
}

impl<G: Generate + ?Sized, T, F: Fn(G::Item) -> T + Clone> Generate for Map<G, F> {
    type Item = T;
    type Shrink = Shrinker<G::Shrink, F>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Shrinker::new(self.inner.generate(state), self.map.clone())
    }
}

impl<S: Shrink, T, F: Fn(S::Item) -> T + Clone> Shrink for Shrinker<S, F> {
    type Item = T;

    fn item(&self) -> Self::Item {
        (self.map)(self.inner.item())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self::new(self.inner.shrink()?, self.map.clone()))
    }
}
