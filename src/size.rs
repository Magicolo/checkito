use crate::generate::{Generate, State};

pub struct Size<G, F = fn(f64) -> f64>(pub(crate) G, pub(crate) F);

impl<G: Generate, F: Fn(f64) -> f64> Generate for Size<G, F> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let old = state.size.0;
        let new = self.1(old).clamp(0.0, 1.0);
        assert!(new.is_finite());
        state.size.0 = new;
        let shrinker = self.0.generate(state);
        state.size.0 = old;
        shrinker
    }

    fn constant(&self) -> bool {
        self.0.constant()
    }
}
