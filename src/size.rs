use crate::{
    generate::{Generate, State},
    IntoShrink,
};

pub struct Size<G, F = fn(f64) -> f64>(pub G, pub F);

impl<G: Generate, F: Fn(f64) -> f64> Generate for Size<G, F> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let old = state.size.clone();
        let new = self.1(old.0).clamp(old.1.start, old.1.end);
        assert!(new.is_finite());
        state.size = (new, old.1.clone());
        let shrink = self.0.generate(state);
        state.size = old;
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
