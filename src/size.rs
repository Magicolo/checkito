use crate::{
    check::Sizes,
    generate::{Generate, State},
};

#[derive(Debug, Clone)]
pub struct Size<G, F>(pub(crate) G, pub(crate) F);

impl<G: Generate, S: Into<Sizes>, F: Fn(Sizes) -> S> Generate for Size<G, F> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let old = state.size;
        let new = self.1(old).into();
        state.size = new;
        let shrinker = self.0.generate(state);
        state.size = old;
        shrinker
    }

    fn constant(&self) -> bool {
        self.0.constant()
    }
}
