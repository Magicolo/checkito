use crate::{
    generate::{Generator, State},
    shrink::Shrinker,
};

#[derive(Clone, Debug, Default)]
pub struct Same<T: ?Sized>(pub(crate) T);

impl<T: Clone> Generator for Same<T> {
    type Item = T;
    type Shrink = Self;

    fn generate(&self, _: &mut State) -> Self::Shrink {
        self.clone()
    }

    fn constant(&self) -> bool {
        true
    }
}

impl<T: Clone> Shrinker for Same<T> {
    type Item = T;

    fn item(&self) -> Self::Item {
        self.0.clone()
    }

    fn shrink(&mut self) -> Option<Self> {
        None
    }
}
