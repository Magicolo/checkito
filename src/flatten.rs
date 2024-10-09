use crate::{
    IntoGenerator,
    generate::{Generator, State},
    shrink::Shrinker,
};

#[derive(Clone, Debug)]
pub struct Flatten<G: ?Sized>(pub(crate) G);

#[derive(Clone, Debug)]
pub struct Shrink<I, O> {
    state: State,
    inner: I,
    outer: O,
}

impl<G: Generator, I: IntoGenerator<IntoGen = G>, O: Generator<Item = I> + ?Sized> Generator
    for Flatten<O>
{
    type Item = G::Item;
    type Shrink = Shrink<G::Shrink, O::Shrink>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let old = state.clone();
        let outer = self.0.generate(state);
        let generator = outer.item().into_gen();
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

impl<G: Generator, I: IntoGenerator<IntoGen = G>, O: Shrinker<Item = I>> Shrinker
    for Shrink<G::Shrink, O>
{
    type Item = G::Item;

    fn item(&self) -> Self::Item {
        self.inner.item()
    }

    fn shrink(&mut self) -> Option<Self> {
        if let Some(outer) = self.outer.shrink() {
            let mut state = self.state.clone();
            let inner = outer.item().into_gen().generate(&mut state);
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
