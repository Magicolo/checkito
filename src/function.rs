use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};

impl<T> Generate for fn() -> T {
    type Item = T;
    type Shrink = Self;

    fn generate(&self, _: &mut State) -> (Self::Item, Self::Shrink) {
        (self(), self.clone())
    }
}

impl<T> Shrink for fn() -> T {
    type Item = T;

    fn generate(&self) -> Self::Item {
        self()
    }

    fn shrink(&mut self) -> Option<Self> {
        None
    }
}
