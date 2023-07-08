use std::marker::PhantomData;

use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};

#[derive(Debug, Default)]
pub struct Map<I: ?Sized, T: ?Sized, F = fn(<I as Generate>::Item) -> T> {
    _marker: PhantomData<T>,
    map: F,
    inner: I,
}

#[derive(Debug)]
pub struct Shrinker<I: ?Sized, T: ?Sized, F> {
    _marker: PhantomData<T>,
    map: F,
    inner: I,
}

impl<I: Clone, T, F: Clone> Clone for Map<I, T, F> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            map: self.map.clone(),
            _marker: PhantomData,
        }
    }
}

impl<I: Clone, T, F: Clone> Clone for Shrinker<I, T, F> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            map: self.map.clone(),
            _marker: PhantomData,
        }
    }
}

impl<G: Generate, T, F: Fn(G::Item) -> T> Map<G, T, F> {
    pub const fn new(generate: G, map: F) -> Self {
        Self {
            inner: generate,
            map,
            _marker: PhantomData,
        }
    }
}

impl<S: Shrink, T, F: Fn(S::Item) -> T> Shrinker<S, T, F> {
    pub const fn new(shrink: S, map: F) -> Self {
        Self {
            inner: shrink,
            map,
            _marker: PhantomData,
        }
    }
}

impl<G: Generate + ?Sized, T, F: Fn(G::Item) -> T + Clone> Generate for Map<G, T, F> {
    type Item = T;
    type Shrink = Shrinker<G::Shrink, T, F>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Shrinker::new(self.inner.generate(state), self.map.clone())
    }
}

impl<S: Shrink, T, F: Fn(S::Item) -> T + Clone> Shrink for Shrinker<S, T, F> {
    type Item = T;

    fn item(&self) -> Self::Item {
        (self.map)(self.inner.item())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self::new(self.inner.shrink()?, self.map.clone()))
    }
}
