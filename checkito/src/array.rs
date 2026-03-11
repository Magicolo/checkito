use crate::{all, cardinality, generate::Generate, primitive::Constant, state::State};
use core::array;

#[derive(Clone, Debug)]
pub struct Array<G: ?Sized, const N: usize>(pub(crate) G);

impl<G: Generate + ?Sized, const N: usize> Generate for Array<G, N> {
    type Item = [G::Item; N];
    type Shrink = all::Shrinker<[G::Shrink; N]>;

    const CARDINALITY: Option<u128> = cardinality::all_repeat_static::<N>(G::CARDINALITY);

    fn generate(&self, state: &mut State) -> Self::Shrink {
        all::Shrinker::new(array::from_fn(|_| self.0.generate(state)))
    }

    fn cardinality(&self) -> Option<u128> {
        cardinality::all_repeat_static::<N>(self.0.cardinality())
    }
}

impl<C: Constant, const N: usize> Constant for Array<C, N> {
    const VALUE: Self = Self(C::VALUE);
}
