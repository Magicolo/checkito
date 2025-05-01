use crate::{generate::Generate, shrink::Shrink, state::State};
use core::marker::PhantomData;

#[derive(Debug)]
pub struct Convert<T: ?Sized, I: ?Sized>(pub(crate) PhantomData<I>, pub(crate) T);

impl<T: Clone, I> Clone for Convert<T, I> {
    fn clone(&self) -> Self {
        Self(PhantomData, self.1.clone())
    }
}

impl<G: Generate + ?Sized, I: From<G::Item>> Generate for Convert<G, I> {
    type Item = I;
    type Shrink = Convert<G::Shrink, I>;

    const CARDINALITY: Option<usize> = G::CARDINALITY;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Convert(PhantomData, self.1.generate(state))
    }

    fn cardinality(&self) -> Option<usize> {
        self.1.cardinality()
    }
}

impl<S: Shrink, I: From<S::Item>> Shrink for Convert<S, I> {
    type Item = I;

    fn item(&self) -> Self::Item {
        I::from(self.1.item())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self(PhantomData, self.1.shrink()?))
    }
}
