use crate::{generate::Generate, shrink::Shrink, state::State};

#[derive(Clone, Debug)]
pub struct Same<T: ?Sized>(pub(crate) T);

impl<T: Clone> Generate for Same<T> {
    type Item = T;
    type Shrink = Self;

    const CARDINALITY: Option<u128> = Some(1);

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
