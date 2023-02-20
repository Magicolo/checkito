use crate::generate::{Generate, State};

#[derive(Clone, Debug, Default)]
pub struct Filter<I: ?Sized, F = fn(&<I as Generate>::Item) -> bool> {
    filter: F,
    iterations: usize,
    inner: I,
}

impl<G: Generate, F: Fn(&G::Item) -> bool> Filter<G, F> {
    #[inline]
    pub fn new(generate: G, filter: F, iterations: usize) -> Self {
        Self {
            inner: generate,
            filter,
            iterations,
        }
    }
}

impl<G: Generate + ?Sized, F: Fn(&G::Item) -> bool + Clone> Generate for Filter<G, F> {
    type Item = Option<G::Item>;
    type Shrink = Option<G::Shrink>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        for _ in 0..self.iterations {
            let (item, shrink) = self.inner.generate(state);
            if (self.filter)(&item) {
                return (Some(item), Some(shrink));
            }
        }
        (None, None)
    }
}
