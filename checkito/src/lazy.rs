use crate::{Generate, state::State};
use std::sync::OnceLock;

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
        self.0.get().map_or(G::CARDINALITY, G::cardinality)
    }
}

/// Implement Generate for lazy initialization types that share the same behavior
macro_rules! impl_lazy_generate {
    ($lazy_type:ty, $force_method:path) => {
        impl<G: Generate, F: FnOnce() -> G> Generate for $lazy_type {
            type Item = G::Item;
            type Shrink = G::Shrink;

            const CARDINALITY: Option<u128> = G::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                $force_method(self).generate(state)
            }

            fn cardinality(&self) -> Option<u128> {
                $force_method(self).cardinality()
            }
        }
    };
}

impl_lazy_generate!(core::cell::LazyCell<G, F>, core::cell::LazyCell::force);
impl_lazy_generate!(std::sync::LazyLock<G, F>, std::sync::LazyLock::force);
