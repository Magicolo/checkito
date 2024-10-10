use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};

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

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let old = state.clone();
        let outer = self.0.generate(state);
        let generator = outer.item();
        state.limit += 1;
        state.depth += 1;
        let inner = generator.generate(state);
        state.depth -= 1;
        Shrinker {
            state: old,
            inner,
            outer,
        }
    }

    fn constant(&self) -> bool {
        false
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
