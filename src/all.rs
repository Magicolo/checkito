use crate::{
    generate::{FullGenerator, Generator, State},
    shrink::Shrinker,
    utility::tuples,
};

#[derive(Clone, Debug)]
pub struct Shrink<S: ?Sized> {
    pub(crate) index: usize,
    pub(crate) shrinkers: S,
}

pub(crate) fn shrink<S: Shrinker, I: AsMut<[S]> + Clone>(
    shrinkers: &mut I,
    index: &mut usize,
) -> Option<I> {
    loop {
        let old = shrinkers.as_mut().get_mut(*index)?;
        if let Some(new) = old.shrink() {
            let mut shrinkers = shrinkers.clone();
            shrinkers.as_mut()[*index] = new;
            break Some(shrinkers);
        } else {
            *index += 1;
        }
    }
}

pub mod array {
    use super::*;
    use core::array;

    impl<G: FullGenerator, const N: usize> FullGenerator for [G; N] {
        type Generator = [G::Generator; N];
        type Item = [G::Item; N];

        fn generator() -> Self::Generator {
            array::from_fn(|_| G::generator())
        }
    }

    impl<G: Generator, const N: usize> Generator for [G; N] {
        type Item = [G::Item; N];
        type Shrink = Shrink<[G::Shrink; N]>;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrink {
                index: 0,
                shrinkers: array::from_fn(|index| self[index].generate(state)),
            }
        }

        fn constant(&self) -> bool {
            self.iter().all(Generator::constant)
        }
    }

    impl<S: Shrinker, const N: usize> Shrinker for Shrink<[S; N]> {
        type Item = [S::Item; N];

        fn item(&self) -> Self::Item {
            array::from_fn(|index| self.shrinkers[index].item())
        }

        fn shrink(&mut self) -> Option<Self> {
            let shrinkers = shrink(&mut self.shrinkers, &mut self.index)?;
            Some(Self {
                shrinkers,
                index: self.index,
            })
        }
    }
}

pub mod slice {
    use super::*;

    impl<G: Generator> Generator for [G] {
        type Item = Box<[G::Item]>;
        type Shrink = Shrink<Box<[G::Shrink]>>;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrink {
                index: 0,
                shrinkers: self
                    .iter()
                    .map(|generator| generator.generate(state))
                    .collect(),
            }
        }

        fn constant(&self) -> bool {
            self.iter().all(|generator| generator.constant())
        }
    }

    impl<S: Shrinker> Shrinker for Shrink<Box<[S]>> {
        type Item = Box<[S::Item]>;

        fn item(&self) -> Self::Item {
            self.shrinkers.iter().map(S::item).collect()
        }

        fn shrink(&mut self) -> Option<Self> {
            let shrinkers = shrink(&mut self.shrinkers, &mut self.index)?;
            Some(Self {
                shrinkers,
                index: self.index,
            })
        }
    }
}

pub mod vector {
    use super::*;

    impl<G: Generator> Generator for Vec<G> {
        type Item = Vec<G::Item>;
        type Shrink = Shrink<Vec<G::Shrink>>;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrink {
                index: 0,
                shrinkers: self
                    .iter()
                    .map(|generator| generator.generate(state))
                    .collect(),
            }
        }

        fn constant(&self) -> bool {
            self.iter().all(|generator| generator.constant())
        }
    }

    impl<S: Shrinker> Shrinker for Shrink<Vec<S>> {
        type Item = Vec<S::Item>;

        fn item(&self) -> Self::Item {
            self.shrinkers.iter().map(S::item).collect()
        }

        fn shrink(&mut self) -> Option<Self> {
            let shrinkers = shrink(&mut self.shrinkers, &mut self.index)?;
            Some(Self {
                shrinkers,
                index: self.index,
            })
        }
    }
}

macro_rules! tuple {
    ($n:ident, $c:tt $(,$p:ident, $t:ident, $i:tt)*) => {
        impl<$($t: FullGenerator,)*> FullGenerator for ($($t,)*) {
            type Generator = ($($t::Generator,)*);
            type Item = ($($t::Item,)*);

            #[allow(clippy::unused_unit)]
            fn generator() -> Self::Generator {
                ($($t::generator(),)*)
            }
        }

        impl<$($t: Generator,)*> Generator for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type Shrink = Shrink<($($t::Shrink,)*)>;

            fn generate(&self, _state: &mut State) -> Self::Shrink {
                Shrink {
                    index: 0,
                    shrinkers: ($($t::generate(&self.$i, _state),)*),
                }
            }

            fn constant(&self) -> bool {
                $($t::constant(&self.$i) &&)* true
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
