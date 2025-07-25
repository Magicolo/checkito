use crate::{cardinality, generate::Generate, shrink::Shrink, state::State};

#[derive(Clone, Debug)]
pub struct Flatten<G: ?Sized>(pub(crate) G);

#[derive(Clone, Debug)]
pub struct Shrinker<I, O> {
    state: State,
    inner: I,
    outer: O,
}

impl<I: Generate, O: Generate<Item = I> + ?Sized> Generate for Flatten<O> {
    type Item = I::Item;
    type Shrink = Shrinker<I::Shrink, O::Shrink>;

    const CARDINALITY: Option<u128> = cardinality::all_product(O::CARDINALITY, I::CARDINALITY);

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let old = state.clone();
        let outer = self.0.generate(state);
        let generator = outer.item();
        let inner = generator.generate(state.descend().as_mut());
        Shrinker {
            state: old,
            inner,
            outer,
        }
    }

    fn cardinality(&self) -> Option<u128> {
        cardinality::all_product(self.0.cardinality(), I::CARDINALITY)
    }
}

impl<I: Generate, O: Shrink<Item = I>> Shrink for Shrinker<I::Shrink, O> {
    type Item = I::Item;

    fn item(&self) -> Self::Item {
        self.inner.item()
    }

    fn shrink(&mut self) -> Option<Self> {
        if let Some(outer) = self.outer.shrink() {
            let mut state = self.state.clone();
            let inner = outer.item().generate(&mut state);
            return Some(Self {
                state,
                outer,
                inner,
            });
        }

        if let Some(inner) = self.inner.shrink() {
            return Some(Self {
                state: self.state.clone(),
                outer: self.outer.clone(),
                inner,
            });
        }

        None
    }
}
