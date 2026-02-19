use crate::{Generate, state::State};
use core::cell::LazyCell;
use std::sync::{LazyLock, OnceLock};

#[derive(Debug, Clone)]
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

/// Implement Generate for lazy initialization types that share the same
/// behavior
macro_rules! lazy {
    ($type:ty) => {
        impl<G: Generate, F: FnOnce() -> G> Generate for $type {
            type Item = G::Item;
            type Shrink = G::Shrink;

            const CARDINALITY: Option<u128> = G::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Self::force(self).generate(state)
            }

            fn cardinality(&self) -> Option<u128> {
                Self::force(self).cardinality()
            }
        }
    };
}

lazy!(LazyCell<G, F>);
lazy!(LazyLock<G, F>);
