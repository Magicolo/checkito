use crate::{
    convert::Convert,
    generate::{FullGenerator, Generator, State},
    shrink::Shrinker,
};
use core::{marker::PhantomData, mem::take};
use std::{rc::Rc, sync::Arc};

pub mod option {
    use super::*;
    use crate::generate::FullGenerator;

    #[derive(Clone, Debug)]
    pub struct Generate<G>(pub(crate) G);

    #[derive(Clone)]
    pub struct Shrink<S>(bool, Option<S>);

    impl<G: FullGenerator> FullGenerator for Option<G> {
        type FullGen = Generate<G::FullGen>;
        type Item = Option<G::Item>;

        fn full_gen() -> Self::FullGen {
            Generate(G::full_gen())
        }
    }

    impl<G: Generator> Generator for Generate<G> {
        type Item = Option<G::Item>;
        type Shrink = Shrink<G::Shrink>;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            if state.random().bool() {
                Shrink(true, Some(self.0.generate(state)))
            } else {
                Shrink(false, None)
            }
        }

        fn constant(&self) -> bool {
            false
        }
    }

    impl<G: Generator> Generator for Option<G> {
        type Item = Option<G::Item>;
        type Shrink = Shrink<G::Shrink>;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrink(
                true,
                self.as_ref().map(|generator| generator.generate(state)),
            )
        }

        fn constant(&self) -> bool {
            self.as_ref().map_or(true, |generator| generator.constant())
        }
    }

    impl<S: Shrinker> Shrinker for Shrink<S> {
        type Item = Option<S::Item>;

        fn item(&self) -> Self::Item {
            Some(self.1.as_ref()?.item())
        }

        fn shrink(&mut self) -> Option<Self> {
            Some(if take(&mut self.0) {
                Self(false, None)
            } else {
                Self(false, Some(self.1.as_mut()?.shrink()?))
            })
        }
    }
}

pub mod result {
    use super::*;

    #[derive(Clone, Debug)]
    pub struct Generate<T, E>(T, E);
    #[derive(Clone, Debug)]
    pub struct Shrink<T, E>(Result<T, E>);

    impl<T: FullGenerator, E: FullGenerator> FullGenerator for Result<T, E> {
        type FullGen = Generate<T::FullGen, E::FullGen>;
        type Item = Result<T::Item, E::Item>;

        fn full_gen() -> Self::FullGen {
            todo!()
        }
    }

    impl<T: Generator, E: Generator> Generator for Generate<T, E> {
        type Item = Result<T::Item, E::Item>;
        type Shrink = Shrink<T::Shrink, E::Shrink>;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrink(if state.random().bool() {
                Ok(self.0.generate(state))
            } else {
                Err(self.1.generate(state))
            })
        }

        fn constant(&self) -> bool {
            false
        }
    }

    impl<T: Generator, E: Generator> Generator for Result<T, E> {
        type Item = Result<T::Item, E::Item>;
        type Shrink = Shrink<T::Shrink, E::Shrink>;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrink(match self {
                Ok(generator) => Ok(generator.generate(state)),
                Err(generator) => Err(generator.generate(state)),
            })
        }

        fn constant(&self) -> bool {
            match self {
                Ok(generator) => generator.constant(),
                Err(generator) => generator.constant(),
            }
        }
    }

    impl<T: Shrinker, E: Shrinker> Shrinker for Shrink<T, E> {
        type Item = Result<T::Item, E::Item>;

        fn item(&self) -> Self::Item {
            match &self.0 {
                Ok(shrinker) => Ok(shrinker.item()),
                Err(shrinker) => Err(shrinker.item()),
            }
        }

        fn shrink(&mut self) -> Option<Self> {
            Some(Self(match &mut self.0 {
                Ok(shrinker) => Ok(shrinker.shrink()?),
                Err(shrinker) => Err(shrinker.shrink()?),
            }))
        }
    }
}

impl<G: FullGenerator + ?Sized> FullGenerator for Box<G> {
    type FullGen = Convert<G::FullGen, Self::Item>;
    type Item = Box<G::Item>;

    fn full_gen() -> Self::FullGen {
        Convert(PhantomData, G::full_gen())
    }
}

impl<G: Generator + ?Sized> Generator for Box<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn constant(&self) -> bool {
        G::constant(self)
    }
}

impl<G: Generator + ?Sized> Generator for Rc<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn constant(&self) -> bool {
        G::constant(self)
    }
}

impl<G: Generator + ?Sized> Generator for Arc<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn constant(&self) -> bool {
        G::constant(self)
    }
}
