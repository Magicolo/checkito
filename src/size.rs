use crate::{
    generate::Generate,
    state::{Sizes, State},
};

#[derive(Debug, Clone)]
pub struct Size<G, F: ?Sized>(pub(crate) G, pub(crate) F);

impl<G: Generate, S: Into<Sizes>, F: Fn(Sizes) -> S> Generate for Size<G, F> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    const CARDINALITY: Option<u128> = G::CARDINALITY;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let sizes = self.1(state.sizes()).into();
        self.0.generate(state.with().sizes(sizes).as_mut())
    }

    fn cardinality(&self) -> Option<u128> {
        self.0.cardinality()
    }
}
