use crate::{
    generate::{Generate, State},
    shrink::Shrink,
    IntoShrink,
};

#[derive(Clone, Debug, Default)]
pub struct Filter<I: ?Sized, F = fn(&<I as Generate>::Item) -> bool> {
    filter: F,
    retries: usize,
    inner: I,
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
    type Shrink = Option<G::Shrink>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let mut outer = None;
        let old = state.size;
        for i in 0..self.retries {
            let new = old + (1.0 - old) * (i as f64 / self.retries as f64);
            state.size = new.min(1.0).max(0.0);
            let inner = self.inner.generate(state);
            let item = inner.item();
            if (self.filter)(&item) {
                outer = Some(inner);
                break;
            }
        }
        state.size = old;
        outer
    }
}

impl<S: IntoShrink, F: Fn(&S::Item) -> bool + Clone> IntoShrink for Filter<S, F> {
    type Item = S::Item;
    type Shrink = S::Shrink;

    fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
        if (self.filter)(&item) {
            self.inner.shrinker(item)
        } else {
            None
        }
    }
}
