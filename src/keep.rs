use crate::{
    generate::{Generate, State},
    shrink::Shrink,
    IntoShrink,
};

#[derive(Clone, Debug)]
pub struct Keep<T: ?Sized>(pub T);

impl<G: Generate + ?Sized> Generate for Keep<G> {
    type Item = G::Item;
    type Shrink = Keep<G::Shrink>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Keep(self.0.generate(state))
    }
}

impl<S: IntoShrink> IntoShrink for Keep<S> {
    type Item = S::Item;
    type Shrink = Keep<S::Shrink>;

    fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
        Some(Keep(self.0.shrinker(item)?))
    }
}

impl<S: Shrink> Shrink for Keep<S> {
    type Item = S::Item;

    fn item(&self) -> Self::Item {
        self.0.item()
    }

    fn shrink(&mut self) -> Option<Self> {
        None
    }
}
