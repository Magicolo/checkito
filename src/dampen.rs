use crate::generate::{Generate, State};

pub struct Dampen<T: ?Sized> {
    pub force: f64,
    pub inner: T,
}

impl<G: Generate + ?Sized> Generate for Dampen<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let old = state.size;
        let new = old / (state.depth as f64 * self.force).max(1.0);
        debug_assert!(old.is_finite());
        debug_assert!(new.is_finite());
        state.size = new.max(0.0).min(1.0);
        let shrink = self.inner.generate(state);
        state.size = old;
        shrink
    }
}
