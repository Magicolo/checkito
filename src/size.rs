use crate::{
    IntoShrink,
    generate::{Generate, State},
};

pub struct Size<G, F = fn(f64) -> f64>(pub G, pub F);

impl<G: Generate, F: Fn(f64) -> f64> Generate for Size<G, F> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let old = state.size.0;
        let new = self.1(old).clamp(0.0, 1.0);
        assert!(new.is_finite());
        state.size.0 = new;
        let shrink = self.0.generate(state);
        state.size.0 = old;
        shrink
    }

    fn constant(&self) -> bool {
        self.0.constant()
    }
}

impl<S: IntoShrink, F> IntoShrink for Size<S, F> {
    type Item = S::Item;
    type Shrink = S::Shrink;

    fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
        self.0.shrinker(item)
    }
}
