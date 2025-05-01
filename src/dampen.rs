use crate::{generate::Generate, state::State};

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

    const CARDINALITY: Option<usize> = G::CARDINALITY;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        self.generator.generate(
            state
                .dampen(self.deepest, self.limit, self.pressure)
                .as_mut(),
        )
    }

    fn cardinality(&self) -> Option<usize> {
        self.generator.cardinality()
    }
}
