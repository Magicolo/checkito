use crate::generate::{Generate, State};

pub struct Size<G, F>(pub G, pub F);

impl<G: Generate, F: Fn(f64) -> f64> Generate for Size<G, F> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let old = state.size;
        let new = self.1(old);
        state.size = new.max(0.0).min(1.0);
        let shrink = self.0.generate(state);
        state.size = old;
        shrink
    }
}
