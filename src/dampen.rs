use crate::generate::{Generator, State};

pub struct Dampen<T: ?Sized> {
    pub pressure: f64,
    pub deepest: usize,
    pub limit: usize,
    pub generator: T,
}

impl<G: Generator + ?Sized> Generator for Dampen<G> {
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
        let shrinker = self.generator.generate(state);
        state.size.0 = old;
        shrinker
    }

    fn constant(&self) -> bool {
        self.generator.constant()
    }
}
