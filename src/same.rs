use crate::{
    collect::Count,
    generate::Generate,
    shrink::Shrink,
    state::{Range, State},
};

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

impl Count for Same<usize> {
    fn count(&self) -> Range<usize> {
        Range::from(self.0)
    }
}
