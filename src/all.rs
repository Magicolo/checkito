use crate::{
    any::Any,
    generate::{FullGenerator, Generator, IntoGenerator, State},
    shrink::Shrinker,
    utility::tuples,
};
use ref_cast::RefCast;

#[repr(transparent)]
#[derive(Clone, Debug, RefCast)]
pub struct All<G: ?Sized>(pub(crate) G);

#[derive(Clone, Debug)]
pub struct Shrink<S: ?Sized> {
    pub(crate) index: usize,
    pub(crate) shrinkers: S,
}

impl<G: ?Sized> Generator for All<All<G>>
where
    All<G>: Generator,
{
    type Item = <All<G> as Generator>::Item;
    type Shrink = <All<G> as Generator>::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        All::ref_cast(&self.0.0).generate(state)
    }

    fn constant(&self) -> bool {
        All::ref_cast(&self.0.0).constant()
    }
}

impl<G: ?Sized> Generator for All<Any<G>>
where
    All<G>: Generator,
{
    type Item = <All<G> as Generator>::Item;
    type Shrink = <All<G> as Generator>::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        All::ref_cast(&self.0.0).generate(state)
    }

    fn constant(&self) -> bool {
        All::ref_cast(&self.0.0).constant()
    }
}

impl<G: ?Sized> Generator for All<&G>
where
    All<G>: Generator,
{
    type Item = <All<G> as Generator>::Item;
    type Shrink = <All<G> as Generator>::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        All::ref_cast(self.0).generate(state)
    }

    fn constant(&self) -> bool {
        All::ref_cast(self.0).constant()
    }
}

impl<G: ?Sized> Generator for All<&mut G>
where
    All<G>: Generator,
{
    type Item = <All<G> as Generator>::Item;
    type Shrink = <All<G> as Generator>::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        All::ref_cast(self.0).generate(state)
    }

    fn constant(&self) -> bool {
        All::ref_cast(self.0).constant()
    }
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

    impl<'a, G: Generator> IntoGenerator for &'a [G] {
        type IntoGen = All<&'a [G]>;
        type Item = Box<[G::Item]>;

        fn into_gen(self) -> Self::IntoGen {
            All(self)
        }
    }

    impl<'a, G: Generator> IntoGenerator for &'a mut [G] {
        type IntoGen = All<&'a mut [G]>;
        type Item = Box<[G::Item]>;

        fn into_gen(self) -> Self::IntoGen {
            All(self)
        }
    }

    impl<G: Generator> Generator for All<[G]> {
        type Item = Box<[G::Item]>;
        type Shrink = Shrink<Box<[G::Shrink]>>;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrink {
                index: 0,
                shrinkers: self
                    .0
                    .iter()
                    .map(|generator| generator.generate(state))
                    .collect(),
            }
        }

        fn constant(&self) -> bool {
            self.0.iter().all(|generator| generator.constant())
        }
    }

    impl<G: IntoGenerator> IntoGenerator for Box<[G]> {
        type IntoGen = All<Box<[G::IntoGen]>>;
        type Item = Box<[G::Item]>;

        fn into_gen(self) -> Self::IntoGen {
            All(Box::into_iter(self).map(G::into_gen).collect())
        }
    }

    impl<G: Generator> Generator for All<Box<[G]>> {
        type Item = Box<[G::Item]>;
        type Shrink = Shrink<Box<[G::Shrink]>>;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrink {
                index: 0,
                shrinkers: self
                    .0
                    .iter()
                    .map(|generator| generator.generate(state))
                    .collect(),
            }
        }

        fn constant(&self) -> bool {
            self.0.iter().all(|generator| generator.constant())
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

    impl<G: IntoGenerator> IntoGenerator for Vec<G> {
        type IntoGen = All<Vec<G::IntoGen>>;
        type Item = Vec<G::Item>;

        fn into_gen(self) -> Self::IntoGen {
            All(Vec::into_iter(self).map(G::into_gen).collect())
        }
    }

    impl<G: Generator> Generator for All<Vec<G>> {
        type Item = Vec<G::Item>;
        type Shrink = Shrink<Vec<G::Shrink>>;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrink {
                index: 0,
                shrinkers: self
                    .0
                    .iter()
                    .map(|generator| generator.generate(state))
                    .collect(),
            }
        }

        fn constant(&self) -> bool {
            self.0.iter().all(|generator| generator.constant())
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
            type FullGen = All<($($t::FullGen,)*)>;
            type Item = ($($t::Item,)*);

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
                $($t::constant(&self.0.$i) &&)* true
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
