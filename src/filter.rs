use crate::{
    IntoShrink,
    generate::{self, Generate, State},
    shrink::Shrink,
};

#[derive(Clone, Debug, Default)]
pub struct Filter<I: ?Sized, F> {
    filter: F,
    retries: usize,
    inner: I,
}

#[derive(Clone, Debug, Default)]
pub struct Shrinker<I, F> {
    inner: Option<I>,
    filter: F,
}

impl<G: Generate, F: Fn(&G::Item) -> bool> Filter<G, F> {
    pub const fn new(generate: G, filter: F, retries: usize) -> Self {
        Self {
            inner: generate,
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
        let old = state.size.clone();
        for i in 0..self.retries {
            let new = generate::size(i, self.retries, state.size.1.clone());
            state.size = new;
            let inner = self.inner.generate(state);
            let item = inner.item();
            if self.constant() || (self.filter)(&item) {
                outer = Some(inner);
                break;
            }
        }
        state.size = old;
        Shrinker {
            inner: outer,
            filter: self.filter.clone(),
        }
    }

    fn constant(&self) -> bool {
        self.inner.constant()
    }
}

impl<S: IntoShrink, F: Fn(&S::Item) -> bool + Clone> IntoShrink for Filter<S, F> {
    type Item = Option<S::Item>;
    type Shrink = Shrinker<S::Shrink, F>;

    fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
        Some(Shrinker {
            filter: self.filter.clone(),
            inner: self.inner.shrinker(item?),
        })
    }
}

impl<S: Shrink, F: Fn(&S::Item) -> bool + Clone> Shrink for Shrinker<S, F> {
    type Item = Option<S::Item>;

    fn item(&self) -> Self::Item {
        self.inner.item().filter(&self.filter)
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Shrinker {
            filter: self.filter.clone(),
            inner: self.inner.shrink()?,
        })
    }
}
