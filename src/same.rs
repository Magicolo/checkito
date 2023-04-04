use crate::{
    generate::{Generate, State},
    shrink::Shrink,
    FullGenerate,
};

#[derive(Clone, Debug, Default)]
pub struct Same<T: ?Sized>(pub T);

impl<T: Default + Clone> FullGenerate for Same<T> {
    type Item = T;
    type Generate = Same<T>;

    fn generator() -> Self::Generate {
        Same(T::default())
    }
}

impl<T: Clone> Generate for Same<T> {
    type Item = T;
    type Shrink = Self;

    fn generate(&self, _: &mut State) -> Self::Shrink {
        self.clone()
    }
}

impl<T: Clone> Shrink for Same<T> {
    type Item = T;

    fn item(&self) -> Self::Item {
        self.0.clone()
    }

    fn shrink(&mut self) -> Option<Self> {
        None
    }
}
