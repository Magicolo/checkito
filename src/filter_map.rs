use std::marker::PhantomData;

use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};

#[derive(Debug, Default)]
pub struct FilterMap<I: ?Sized, T: ?Sized, F = fn(<I as Generate>::Item) -> Option<T>> {
    _marker: PhantomData<T>,
    map: F,
    retries: usize,
    inner: I,
}

#[derive(Debug)]
pub struct Shrinker<I, T: ?Sized, F = fn(<I as Shrink>::Item) -> Option<T>> {
    inner: Option<I>,
    map: F,
    _marker: PhantomData<T>,
}

impl<I: Clone, T, F: Clone> Clone for FilterMap<I, T, F> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            map: self.map.clone(),
            retries: self.retries,
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

impl<G: Generate, T, F: Fn(G::Item) -> Option<T>> FilterMap<G, T, F> {
    pub const fn new(generate: G, map: F, retries: usize) -> Self {
        Self {
            inner: generate,
            map,
            retries,
            _marker: PhantomData,
        }
    }
}

impl<G: Generate + ?Sized, T, F: Fn(G::Item) -> Option<T> + Clone> Generate for FilterMap<G, T, F> {
    type Item = Option<T>;
    type Shrink = Shrinker<G::Shrink, T, F>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let mut outer = Shrinker {
            inner: None,
            map: self.map.clone(),
            _marker: PhantomData,
        };
        let old = state.size;
        for i in 0..self.retries {
            let new = old + (1.0 - old) * (i as f64 / self.retries as f64);
            state.size = new.min(1.0).max(0.0);
            let inner = self.inner.generate(state);
            let item = inner.item();
            if let Some(_) = (self.map)(item) {
                outer.inner = Some(inner);
                break;
            }
        }
        state.size = old;
        outer
    }
}

impl<S: Shrink + ?Sized, T, F: Fn(S::Item) -> Option<T> + Clone> Shrink for Shrinker<S, T, F> {
    type Item = Option<T>;

    fn item(&self) -> Self::Item {
        (self.map)(self.inner.item()?)
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self {
            inner: self.inner.shrink()?,
            map: self.map.clone(),
            _marker: PhantomData,
        })
    }
}
