use crate::{
    generate::Generate,
    shrink::Shrink,
    state::{Sizes, State},
};

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

    const CARDINALITY: Option<usize> = G::CARDINALITY;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let mut outer = None;
        for i in 0..=self.retries {
            let sizes = Sizes::from_ratio(i, self.retries, state.sizes());
            let inner = self.generator.generate(state.with().sizes(sizes).as_mut());
            let item = inner.item();
            if (self.filter)(&item) {
                outer = Some(inner);
                break;
            }
        }
        Shrinker {
            shrinker: outer,
            filter: self.filter.clone(),
        }
    }

    fn cardinality(&self) -> Option<usize> {
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
