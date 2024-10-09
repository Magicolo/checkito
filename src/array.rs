use crate::{
    all,
    generate::{Generator, State},
};
use core::array;

#[derive(Clone, Debug, Default)]
pub struct Array<G: ?Sized, const N: usize>(pub(crate) G);

impl<G: Generator + ?Sized, const N: usize> Generator for Array<G, N> {
    type Item = [G::Item; N];
    type Shrink = all::Shrink<[G::Shrink; N]>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        all::Shrink {
            index: 0,
            shrinkers: array::from_fn(|_| self.0.generate(state)),
        }
    }

    fn constant(&self) -> bool {
        N == 0 || self.0.constant()
    }
}
