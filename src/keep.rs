use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};

#[derive(Clone, Debug)]
pub struct Keep<T: ?Sized>(pub T);

impl<G: Generate + ?Sized> Generate for Keep<G> {
    type Item = G::Item;
    type Shrink = Keep<G::Shrink>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        let (item, shrink) = self.0.generate(state);
        (item, Keep(shrink))
    }
}

impl<S: Shrink> Shrink for Keep<S> {
    type Item = S::Item;

    fn generate(&self) -> Self::Item {
        self.0.generate()
    }

    fn shrink(&mut self) -> Option<Self> {
        None
    }
}
