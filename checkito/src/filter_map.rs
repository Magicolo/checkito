use crate::{
    generate::Generate,
    shrink::Shrink,
    state::{Sizes, State},
};

#[derive(Debug, Clone)]
pub struct FilterMap<G: ?Sized, F> {
    pub(crate) filter: F,
    pub(crate) retries: usize,
    pub(crate) generator: G,
}

#[derive(Debug, Clone)]
pub struct Shrinker<S, F> {
    shrinker: Option<S>,
    map: F,
}

impl<G: Generate + ?Sized, T, F: Fn(G::Item) -> Option<T> + Clone> Generate for FilterMap<G, F> {
    type Item = Option<T>;
    type Shrink = Shrinker<G::Shrink, F>;

    const CARDINALITY: Option<u128> = G::CARDINALITY;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let mut outer = None;
        for i in 0..=self.retries {
            let sizes = Sizes::from_ratio(i, self.retries, state.sizes());
            let inner = self.generator.generate(state.with().sizes(sizes).as_mut());
            let item = inner.item();
            if (self.filter)(item).is_some() {
                outer = Some(inner);
                break;
            }
        }
        Shrinker {
            shrinker: outer,
            map: self.filter.clone(),
        }
    }

    fn cardinality(&self) -> Option<u128> {
        self.generator.cardinality()
    }
}

impl<S: Shrink, T, F: Fn(S::Item) -> Option<T> + Clone> Shrink for Shrinker<S, F> {
    type Item = Option<T>;

    fn item(&self) -> Self::Item {
        (self.map)(self.shrinker.as_ref()?.item())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self {
            shrinker: Some(self.shrinker.as_mut()?.shrink()?),
            map: self.map.clone(),
        })
    }
}
