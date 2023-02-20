use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};

impl<G: Generate> Generate for Option<G> {
    type Item = Option<G::Item>;
    type Shrink = Option<G::Shrink>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        match self {
            Some(generate) => {
                let (item, shrink) = generate.generate(state);
                (Some(item), Some(shrink))
            }
            None => (None, None),
        }
    }
}

impl<S: Shrink> Shrink for Option<S> {
    type Item = Option<S::Item>;

    fn generate(&self) -> Self::Item {
        Some(self.as_ref()?.generate())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Some(self.as_mut()?.shrink()?))
    }
}
