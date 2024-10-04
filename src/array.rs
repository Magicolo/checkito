use crate::{
    FullGenerator, IntoGenerator,
    generate::{Generator, State},
    shrink::{All, Shrinker},
};

#[derive(Clone, Debug, Default)]
pub struct Array<T: ?Sized, const N: usize>(pub T);

impl<G: FullGenerator, const N: usize> FullGenerator for Array<G, N> {
    type FullGen = Array<G::FullGen, N>;
    type Item = [G::Item; N];

    fn full_gen() -> Self::FullGen {
        Array(G::full_gen())
    }
}

impl<G: IntoGenerator, const N: usize> IntoGenerator for Array<G, N> {
    type IntoGen = Array<G::IntoGen, N>;
    type Item = [G::Item; N];

    fn into_gen(self) -> Self::IntoGen {
        Array(self.0.into_gen())
    }
}

impl<G: Generator + ?Sized, const N: usize> Generator for Array<G, N> {
    type Item = [G::Item; N];
    type Shrink = All<[G::Shrink; N]>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        All::new([(); N].map(|_| self.0.generate(state)))
    }

    fn constant(&self) -> bool {
        N == 0 || self.0.constant()
    }
}

impl<G: FullGenerator, const N: usize> FullGenerator for [G; N] {
    type FullGen = [G::FullGen; N];
    type Item = [G::Item; N];

    fn full_gen() -> Self::FullGen {
        [(); N].map(|_| G::full_gen())
    }
}

impl<G: IntoGenerator, const N: usize> IntoGenerator for [G; N] {
    type IntoGen = [G::IntoGen; N];
    type Item = [G::Item; N];

    fn into_gen(self) -> Self::IntoGen {
        self.map(|generator| generator.into_gen())
    }
}

impl<G: Generator, const N: usize> Generator for [G; N] {
    type Item = [G::Item; N];
    type Shrink = All<[G::Shrink; N]>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let mut index = 0;
        All::new([(); N].map(|_| {
            let shrinker = self[index].generate(state);
            index += 1;
            shrinker
        }))
    }

    fn constant(&self) -> bool {
        self.iter().all(Generator::constant)
    }
}

impl<S: Shrinker, const N: usize> Shrinker for All<[S; N]> {
    type Item = [S::Item; N];

    fn item(&self) -> Self::Item {
        let mut index = 0;
        [(); N].map(|_| {
            let item = self.items[index].item();
            index += 1;
            item
        })
    }

    fn shrink(&mut self) -> Option<Self> {
        while let Some(old) = self.items.get_mut(self.index) {
            if let Some(new) = old.shrink() {
                let mut items = self.items.clone();
                items[self.index] = new;
                return Some(Self {
                    items,
                    index: self.index,
                });
            } else {
                self.index += 1;
            }
        }
        None
    }
}
