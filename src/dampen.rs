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
        let old = state.size.clone();
        let new = if state.depth as usize >= self.deepest || state.limit as usize >= self.limit {
            0.0
        } else {
            old.0 / (state.depth as f64 * self.pressure).max(1.0)
        }
        .clamp(old.1.start, old.1.end);
        assert!(new.is_finite());
        state.size = (new, old.1.clone());
        let shrink = self.inner.generate(state);
        state.size = old;
        shrink
    }

    fn constant(&self) -> bool {
        self.inner.constant()
    }
}
