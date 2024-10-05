use crate::{
    FullGenerator, IntoGenerator,
    all::{All, Shrink},
    generate::{Generator, State},
};
use core::array;

#[derive(Clone, Debug, Default)]
pub struct Array<G: ?Sized, const N: usize>(pub(crate) G);

impl<G: Generator + ?Sized, const N: usize> Generator for Array<G, N> {
    type Item = [G::Item; N];
    type Shrink = Shrink<[G::Shrink; N]>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Shrink {
            index: 0,
            shrinkers: array::from_fn(|_| self.0.generate(state)),
        }
    }

    fn constant(&self) -> bool {
        N == 0 || self.0.constant()
    }
}

impl<G: FullGenerator, const N: usize> FullGenerator for [G; N] {
    type FullGen = All<[G::FullGen; N]>;
    type Item = [G::Item; N];

    fn full_gen() -> Self::FullGen {
        All(array::from_fn(|_| G::full_gen()))
    }
}

impl<G: IntoGenerator, const N: usize> IntoGenerator for [G; N] {
    type IntoGen = All<[G::IntoGen; N]>;
    type Item = [G::Item; N];

    fn into_gen(self) -> Self::IntoGen {
        All(self.map(IntoGenerator::into_gen))
    }
}
