use crate::{
    check::Sizes,
    generate::{Generate, State},
};

#[derive(Clone, Debug)]
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
        let old = state.size;
        let new = if state.depth as usize >= self.deepest || state.limit as usize >= self.limit {
            0.0
        } else {
            old.start() / (state.depth as f64 * self.pressure).max(1.0)
        };
        state.size = Sizes::from(new..=old.end());
        let shrinker = self.generator.generate(state);
        state.size = old;
        shrinker
    }

    fn constant(&self) -> bool {
        self.generator.constant()
    }
}
