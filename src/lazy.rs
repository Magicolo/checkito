use crate::{Generate, state::State};
use std::sync::OnceLock;

pub struct Lazy<T, F>(OnceLock<T>, F);

impl<G: Generate, F: Fn() -> G> Lazy<G, F> {
    pub const fn new(generator: F) -> Self {
        Self(OnceLock::new(), generator)
    }
}

impl<G: Generate, F: Fn() -> G> Generate for Lazy<G, F> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    const CARDINALITY: Option<u128> = G::CARDINALITY;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        self.0.get_or_init(|| self.1()).generate(state)
    }

    fn cardinality(&self) -> Option<u128> {
        self.0.get().map_or(G::CARDINALITY, G::cardinality)
    }
}

#[rustversion::since(1.80)]
#[allow(clippy::incompatible_msrv)]
impl<G: Generate, F: FnOnce() -> G> Generate for core::cell::LazyCell<G, F> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    const CARDINALITY: Option<u128> = G::CARDINALITY;

    fn generate(&self, state: &mut crate::state::State) -> Self::Shrink {
        Self::force(self).generate(state)
    }

    fn cardinality(&self) -> Option<u128> {
        Self::force(self).cardinality()
    }
}

#[rustversion::since(1.80)]
#[allow(clippy::incompatible_msrv)]
impl<G: Generate, F: FnOnce() -> G> Generate for std::sync::LazyLock<G, F> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    const CARDINALITY: Option<u128> = G::CARDINALITY;

    fn generate(&self, state: &mut crate::state::State) -> Self::Shrink {
        Self::force(self).generate(state)
    }

    fn cardinality(&self) -> Option<u128> {
        Self::force(self).cardinality()
    }
}
