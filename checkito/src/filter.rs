use crate::{generate::Generate, shrink::Shrink, state::State};

#[derive(Clone, Debug)]
pub struct Filter<G: ?Sized, F> {
    pub(crate) filter: F,
    pub(crate) generator: G,
}

#[derive(Clone, Debug)]
pub struct Shrinker<S, F> {
    shrinker: S,
    filter: F,
}

impl<G: Generate + ?Sized, F: Fn(&G::Item) -> bool + Clone> Generate for Filter<G, F> {
    type Item = Option<G::Item>;
    type Shrink = Shrinker<G::Shrink, F>;

    const CARDINALITY: Option<u128> = G::CARDINALITY;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Shrinker {
            shrinker: self.generator.generate(state),
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
        let item = self.shrinker.item();
        if (self.filter)(&item) {
            Some(item)
        } else {
            None
        }
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Shrinker {
            filter: self.filter.clone(),
            shrinker: self.shrinker.shrink()?,
        })
    }
}
