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
        self.0.get_or_init(|| self.1()).cardinality()
    }
}

#[rustversion::since(1.80)]
use core::cell::LazyCell;
#[rustversion::since(1.80)]
#[allow(clippy::incompatible_msrv)]
impl<G: Generate, F: FnOnce() -> G> Generate for LazyCell<G, F> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    const CARDINALITY: Option<u128> = G::CARDINALITY;

    fn generate(&self, state: &mut crate::state::State) -> Self::Shrink {
        LazyCell::force(self).generate(state)
    }

    fn cardinality(&self) -> Option<u128> {
        LazyCell::force(self).cardinality()
    }
}

#[rustversion::since(1.80)]
use std::sync::LazyLock;
#[rustversion::since(1.80)]
#[allow(clippy::incompatible_msrv)]
impl<G: Generate, F: FnOnce() -> G> Generate for LazyLock<G, F> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    const CARDINALITY: Option<u128> = G::CARDINALITY;

    fn generate(&self, state: &mut crate::state::State) -> Self::Shrink {
        LazyLock::force(self).generate(state)
    }

    fn cardinality(&self) -> Option<u128> {
        LazyLock::force(self).cardinality()
    }
}
