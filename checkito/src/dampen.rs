use crate::{generate::Generate, state::State, utility};

#[derive(Clone, Debug)]
pub struct Dampen<G: ?Sized> {
    pressure: f64,
    deepest: usize,
    limit: usize,
    generator: G,
}

impl<G> Dampen<G> {
    pub const fn new(pressure: f64, deepest: usize, limit: usize, generator: G) -> Self {
        assert!(pressure.is_finite());
        Self {
            pressure: utility::f64::max(pressure, 0.0),
            deepest,
            limit,
            generator,
        }
    }
}

impl<G: Generate + ?Sized> Generate for Dampen<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    const CARDINALITY: Option<u128> = G::CARDINALITY;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        self.generator.generate(
            state
                .dampen(self.deepest, self.limit, self.pressure)
                .as_mut(),
        )
    }

    fn cardinality(&self) -> Option<u128> {
        self.generator.cardinality()
    }
}
