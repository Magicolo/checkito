use crate::{
    generate::{Generator, State},
    shrink::Shrinker,
};

#[derive(Clone, Debug, Default)]
pub struct Flatten<T: ?Sized>(pub T);

#[derive(Clone, Debug)]
pub struct Shrink<I, O> {
    state: State,
    inner: I,
    outer: O,
}

impl<G: Generator<Item = impl Generator> + ?Sized> Generator for Flatten<G> {
    type Item = <G::Item as Generator>::Item;
    type Shrink = Shrink<<G::Item as Generator>::Shrink, G::Shrink>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let old = state.clone();
        let outer = self.0.generate(state);
        let generator = outer.item();
        state.limit += 1;
        state.depth += 1;
        let inner = generator.generate(state);
        state.depth -= 1;
        Shrink {
            state: old,
            inner,
            outer,
        }
    }

    fn constant(&self) -> bool {
        false
    }
}

impl<I: Shrinker, O: Shrinker<Item = impl Generator<Shrink = I>>> Shrinker for Shrink<I, O> {
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
