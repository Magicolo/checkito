use crate::{
    convert::Convert,
    generate::{FullGenerate, Generate, State},
    shrink::Shrink,
};
use core::{marker::PhantomData, mem::take};
use std::{rc::Rc, sync::Arc};

pub mod option {
    use super::*;
    use crate::generate::FullGenerate;

    #[derive(Clone, Debug)]
    pub struct Generatez<G>(pub(crate) G);

    #[derive(Clone)]
    pub struct Shrinkz<S>(bool, Option<S>);

    impl<G: FullGenerate> FullGenerate for Option<G> {
        type Generator = Generatez<G::Generator>;
        type Item = Option<G::Item>;

        fn generator() -> Self::Generator {
            Generatez(G::generator())
        }
    }

    impl<G: Generate> Generate for Generatez<G> {
        type Item = Option<G::Item>;
        type Shrink = Shrinkz<G::Shrink>;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            if state.random().bool() {
                Shrinkz(true, Some(self.0.generate(state)))
            } else {
                Shrinkz(false, None)
            }
        }

        fn constant(&self) -> bool {
            false
        }
    }

    impl<G: Generate> Generate for Option<G> {
        type Item = Option<G::Item>;
        type Shrink = Shrinkz<G::Shrink>;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrinkz(
                true,
                self.as_ref().map(|generator| generator.generate(state)),
            )
        }

        fn constant(&self) -> bool {
            self.as_ref().map_or(true, |generator| generator.constant())
        }
    }

    impl<S: Shrink> Shrink for Shrinkz<S> {
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
    pub struct Generatez<T, E>(T, E);
    #[derive(Clone, Debug)]
    pub struct Shrinkz<T, E>(Result<T, E>);

    impl<T: FullGenerate, E: FullGenerate> FullGenerate for Result<T, E> {
        type Generator = Generatez<T::Generator, E::Generator>;
        type Item = Result<T::Item, E::Item>;

        fn generator() -> Self::Generator {
            todo!()
        }
    }

    impl<T: Generate, E: Generate> Generate for Generatez<T, E> {
        type Item = Result<T::Item, E::Item>;
        type Shrink = Shrinkz<T::Shrink, E::Shrink>;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrinkz(if state.random().bool() {
                Ok(self.0.generate(state))
            } else {
                Err(self.1.generate(state))
            })
        }

        fn constant(&self) -> bool {
            false
        }
    }

    impl<T: Generate, E: Generate> Generate for Result<T, E> {
        type Item = Result<T::Item, E::Item>;
        type Shrink = Shrinkz<T::Shrink, E::Shrink>;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrinkz(match self {
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

    impl<T: Shrink, E: Shrink> Shrink for Shrinkz<T, E> {
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

impl<G: FullGenerate + ?Sized> FullGenerate for Box<G> {
    type Generator = Convert<G::Generator, Self::Item>;
    type Item = Box<G::Item>;

    fn generator() -> Self::Generator {
        Convert(PhantomData, G::generator())
    }
}

impl<G: Generate + ?Sized> Generate for Box<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn constant(&self) -> bool {
        G::constant(self)
    }
}

impl<G: Generate + ?Sized> Generate for Rc<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn constant(&self) -> bool {
        G::constant(self)
    }
}

impl<G: Generate + ?Sized> Generate for Arc<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn constant(&self) -> bool {
        G::constant(self)
    }
}
