use crate::generate::{Generate, State};

pub struct Dampen<G: ?Sized> {
    pub(crate) pressure: f64,
    pub(crate) deepest: usize,
    pub(crate) limit: usize,
    pub(crate) generator: G,
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
        let shrinker = self.generator.generate(state);
        state.size.0 = old;
        shrinker
    }

    fn constant(&self) -> bool {
        self.generator.constant()
    }
}
