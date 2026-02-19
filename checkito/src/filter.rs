use crate::{
    generate::Generate,
    shrink::Shrink,
    state::{Sizes, State},
};

/// Helper function for retry-based generation with varying sizes.
///
/// This encapsulates the common pattern used in filter/filter_map where
/// we try multiple times to generate a value that satisfies a predicate,
/// increasing the size on each retry attempt.
#[inline]
pub(crate) fn retry_generate<G, F>(
    generator: &G,
    retries: usize,
    state: &mut State,
    mut check: F,
) -> Option<G::Shrink>
where
    G: Generate + ?Sized,
    F: FnMut(&G::Shrink) -> bool,
{
    for i in 0..=retries {
        let sizes = Sizes::from_ratio(i, retries, state.sizes());
        let shrinker = generator.generate(state.with().sizes(sizes).as_mut());
        if check(&shrinker) {
            return Some(shrinker);
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct Filter<G: ?Sized, F> {
    pub(crate) filter: F,
    pub(crate) retries: usize,
    pub(crate) generator: G,
}

#[derive(Clone, Debug)]
pub struct Shrinker<S, F> {
    shrinker: Option<S>,
    filter: F,
}

impl<G: Generate + ?Sized, F: Fn(&G::Item) -> bool + Clone> Generate for Filter<G, F> {
    type Item = Option<G::Item>;
    type Shrink = Shrinker<G::Shrink, F>;

    const CARDINALITY: Option<u128> = G::CARDINALITY;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let filter = &self.filter;
        let shrinker = retry_generate(&self.generator, self.retries, state, |s| {
            filter(&s.item())
        });
        Shrinker {
            shrinker,
            filter: self.filter.clone(),
        }
    }

    fn cardinality(&self) -> Option<u128> {
        self.generator.cardinality()
    }
}

impl<S: Shrink, F: Fn(&S::Item) -> bool + Clone> Shrink for Shrinker<S, F> {
    type Item = Option<S::Item>;

    fn item(&self) -> Self::Item {
        let item = self.shrinker.as_ref()?.item();
        if (self.filter)(&item) {
            Some(item)
        } else {
            None
        }
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Shrinker {
            filter: self.filter.clone(),
            shrinker: Some(self.shrinker.as_mut()?.shrink()?),
        })
    }
}
