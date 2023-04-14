use crate::{
    generate::{Generate, State},
    Shrink,
};

#[derive(Clone, Debug, Default)]
pub struct Flatten<T: ?Sized>(pub T);

#[derive(Clone, Debug)]
pub struct Shrinker<I, O> {
    state: State,
    inner: I,
    outer: O,
}

impl<G: Generate<Item = impl Generate> + ?Sized> Generate for Flatten<G> {
    type Item = <G::Item as Generate>::Item;
    type Shrink = Shrinker<<G::Item as Generate>::Shrink, G::Shrink>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let old = state.clone();
        let outer = self.0.generate(state);
        let item = outer.item();
        state.depth += 1;
        let inner = item.generate(state);
        state.depth -= 1;
        let shrink = Shrinker {
            state: old,
            inner,
            outer,
        };
        shrink
    }
}

impl<I: Shrink, O: Shrink<Item = impl Generate<Shrink = I>>> Shrink for Shrinker<I, O> {
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
