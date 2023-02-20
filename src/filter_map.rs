use std::marker::PhantomData;

use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};

#[derive(Debug, Default)]
pub struct FilterMap<I: ?Sized, T: ?Sized, F = fn(<I as Generate>::Item) -> Option<T>> {
    _marker: PhantomData<T>,
    map: F,
    iterations: usize,
    inner: I,
}

#[derive(Debug, Default)]
pub struct Shrinker<I, T: ?Sized, F = fn(<I as Shrink>::Item) -> Option<T>> {
    inner: Option<I>,
    map: F,
    _marker: PhantomData<T>,
}

impl<I: Clone, T, F: Clone> Clone for FilterMap<I, T, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            map: self.map.clone(),
            iterations: self.iterations,
            _marker: PhantomData,
        }
    }
}

impl<I: Clone, T, F: Clone> Clone for Shrinker<I, T, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            map: self.map.clone(),
            _marker: PhantomData,
        }
    }
}

impl<G: Generate, T, F: Fn(G::Item) -> Option<T>> FilterMap<G, T, F> {
    #[inline]
    pub fn new(generate: G, map: F, iterations: usize) -> Self {
        Self {
            inner: generate,
            map,
            iterations,
            _marker: PhantomData,
        }
    }
}

impl<G: Generate + ?Sized, T, F: Fn(G::Item) -> Option<T> + Clone> Generate for FilterMap<G, T, F> {
    type Item = Option<T>;
    type Shrink = Shrinker<G::Shrink, T, F>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        for _ in 0..self.iterations {
            let (item, shrink) = self.inner.generate(state);
            if let Some(item) = (self.map)(item) {
                return (
                    Some(item),
                    Shrinker {
                        inner: Some(shrink),
                        map: self.map.clone(),
                        _marker: PhantomData,
                    },
                );
            }
        }
        (
            None,
            Shrinker {
                inner: None,
                map: self.map.clone(),
                _marker: PhantomData,
            },
        )
    }
}

impl<S: Shrink + ?Sized, T, F: Fn(S::Item) -> Option<T> + Clone> Shrink for Shrinker<S, T, F> {
    type Item = Option<T>;

    fn generate(&self) -> Self::Item {
        (self.map)(self.inner.generate()?)
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self {
            inner: self.inner.shrink()?,
            map: self.map.clone(),
            _marker: PhantomData,
        })
    }
}
