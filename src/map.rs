use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};

#[derive(Debug, Clone)]
pub struct Map<T: ?Sized, F>(pub(crate) F, pub(crate) T);

impl<G: Generate + ?Sized, T, F: Fn(G::Item) -> T + Clone> Generate for Map<G, F> {
    type Item = T;
    type Shrink = Map<G::Shrink, F>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Map(self.0.clone(), self.1.generate(state))
    }

    fn constant(&self) -> bool {
        self.1.constant()
    }
}

impl<S: Shrink, T, F: Fn(S::Item) -> T + Clone> Shrink for Map<S, F> {
    type Item = T;

    fn item(&self) -> Self::Item {
        self.0(self.1.item())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self(self.0.clone(), self.1.shrink()?))
    }
}
