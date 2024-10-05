use crate::{
    generate::{FullGenerator, Generator, IntoGenerator, State},
    shrink::Shrinker,
    utility::tuples,
};
use core::array;

#[derive(Clone, Debug)]
pub struct All<G: ?Sized>(pub(crate) G);

#[derive(Clone, Debug)]
pub struct Shrink<S: ?Sized> {
    pub(crate) index: usize,
    pub(crate) shrinkers: S,
}

impl<T: ?Sized> AsRef<T> for All<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<G: Generator, const N: usize> Generator for All<[G; N]> {
    type Item = [G::Item; N];
    type Shrink = Shrink<[G::Shrink; N]>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Shrink {
            index: 0,
            shrinkers: array::from_fn(|index| self.0[index].generate(state)),
        }
    }

    fn constant(&self) -> bool {
        self.0.iter().all(Generator::constant)
    }
}

impl<S: Shrinker, const N: usize> Shrinker for Shrink<[S; N]> {
    type Item = [S::Item; N];

    fn item(&self) -> Self::Item {
        array::from_fn(|index| self.shrinkers[index].item())
    }

    fn shrink(&mut self) -> Option<Self> {
        while let Some(old) = self.shrinkers.get_mut(self.index) {
            if let Some(new) = old.shrink() {
                let mut shrinkers = self.shrinkers.clone();
                shrinkers[self.index] = new;
                return Some(Self {
                    shrinkers,
                    index: self.index,
                });
            } else {
                self.index += 1;
            }
        }
        None
    }
}

macro_rules! tuple {
    ($n:ident, $c:tt $(,$p:ident, $t:ident, $i:tt)*) => {
        impl<$($t: FullGenerator,)*> FullGenerator for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type FullGen = All<($($t::FullGen,)*)>;

            #[allow(clippy::unused_unit)]
            fn full_gen() -> Self::FullGen {
                All(($($t::full_gen(),)*))
            }
        }

        impl<$($t: IntoGenerator,)*> IntoGenerator for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type IntoGen = All<($($t::IntoGen,)*)>;

            #[allow(clippy::unused_unit)]
            fn into_gen(self) -> Self::IntoGen {
                All(($(self.$i.into_gen(),)*))
            }
        }

        impl<$($t: Generator,)*> Generator for All<($($t,)*)> {
            type Item = ($($t::Item,)*);
            type Shrink = Shrink<($($t::Shrink,)*)>;

            fn generate(&self, _state: &mut State) -> Self::Shrink {
                Shrink {
                    index: 0,
                    shrinkers: ($($t::generate(&self.0.$i, _state),)*),
                }
            }

            fn constant(&self) -> bool {
                true
            }
        }

        impl<$($t: Shrinker,)*> Shrinker for Shrink<($($t,)*)> {
            type Item = ($($t::Item,)*);

            #[allow(clippy::unused_unit)]
            fn item(&self) -> Self::Item {
                ($(self.shrinkers.$i.item(),)*)
            }

            fn shrink(&mut self) -> Option<Self> {
                loop {
                    match self.index {
                        $($i => {
                            if let Some(shrinker) = self.shrinkers.$i.shrink() {
                                let mut shrinkers = self.shrinkers.clone();
                                shrinkers.$i = shrinker;
                                break Some(Self { shrinkers, index: self.index });
                            } else {
                                self.index += 1;
                            }
                        })*
                        _ => break None,
                    }
                }
            }
        }
    };
}

tuples!(tuple);
