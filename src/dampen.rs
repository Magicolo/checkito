use crate::generate::{Generate, State};

pub struct Dampen<T: ?Sized> {
    pub pressure: f64,
    pub deepest: usize,
    pub limit: usize,
    pub inner: T,
}

impl<G: Generate + ?Sized> Generate for Dampen<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let old = state.size.0;
        let new = if state.depth as usize >= self.deepest || state.limit as usize >= self.limit {
            0.0
        } else {
            old / (state.depth as f64 * self.pressure).max(1.0)
        };
        assert!(new.is_finite());
        state.size.0 = new;
        let shrink = self.inner.generate(state);
        state.size.0 = old;
        shrink
    }

    fn constant(&self) -> bool {
        self.inner.constant()
    }
}
