use crate::{
    all,
    generate::{Generate, State},
};
use core::array;

#[derive(Clone, Debug)]
pub struct Array<G: ?Sized, const N: usize>(pub(crate) G);

impl<G: Generate + ?Sized, const N: usize> Generate for Array<G, N> {
    type Item = [G::Item; N];
    type Shrink = all::Shrinker<[G::Shrink; N]>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        all::Shrinker {
            index: 0,
            shrinkers: array::from_fn(|_| self.0.generate(state)),
        }
    }

    fn constant(&self) -> bool {
        N == 0 || self.0.constant()
    }
}
