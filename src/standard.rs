use crate::{
    cardinality,
    convert::Convert,
    generate::{FullGenerate, Generate},
    shrink::Shrink,
    state::State,
};
use core::{marker::PhantomData, mem::take};
use std::{rc::Rc, sync::Arc};

pub mod option {
    use super::*;
    use crate::generate::FullGenerate;

    #[derive(Clone, Debug)]
    pub struct Generator<G>(pub(crate) G);

    #[derive(Debug, Clone)]
    pub struct Shrinker<S>(bool, Option<S>);

    impl<G: FullGenerate> FullGenerate for Option<G> {
        type Generator = Generator<G::Generator>;
        type Item = Option<G::Item>;

        fn generator() -> Self::Generator {
            Generator(G::generator())
        }
    }

    impl<G: Generate> Generate for Generator<G> {
        type Item = Option<G::Item>;
        type Shrink = Shrinker<G::Shrink>;

        const CARDINALITY: Option<u128> = cardinality::any_sum(G::CARDINALITY, Some(1));

        fn generate(&self, state: &mut State) -> Self::Shrink {
            if state.with().size(1.0).bool() {
                Shrinker(true, Some(self.0.generate(state)))
            } else {
                Shrinker(false, None)
            }
        }

        fn cardinality(&self) -> Option<u128> {
            cardinality::any_sum(self.0.cardinality(), Some(1))
        }
    }

    impl<G: Generate> Generate for Option<G> {
        type Item = Option<G::Item>;
        type Shrink = Shrinker<G::Shrink>;

        const CARDINALITY: Option<u128> = cardinality::any_sum(G::CARDINALITY, Some(1));

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrinker(
                true,
                self.as_ref().map(|generator| generator.generate(state)),
            )
        }

        fn cardinality(&self) -> Option<u128> {
            self.as_ref().map_or(Some(1), Generate::cardinality)
        }
    }

    impl<S: Shrink> Shrink for Shrinker<S> {
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
    pub struct Generator<T, E>(T, E);
    #[derive(Clone, Debug)]
    pub struct Shrinker<T, E>(Result<T, E>);

    impl<T: FullGenerate, E: FullGenerate> FullGenerate for Result<T, E> {
        type Generator = Generator<T::Generator, E::Generator>;
        type Item = Result<T::Item, E::Item>;

        fn generator() -> Self::Generator {
            Generator(T::generator(), E::generator())
        }
    }

    impl<T: Generate, E: Generate> Generate for Generator<T, E> {
        type Item = Result<T::Item, E::Item>;
        type Shrink = Shrinker<T::Shrink, E::Shrink>;

        const CARDINALITY: Option<u128> = cardinality::any_sum(T::CARDINALITY, E::CARDINALITY);

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrinker(if state.with().size(1.0).bool() {
                Ok(self.0.generate(state))
            } else {
                Err(self.1.generate(state))
            })
        }

        fn cardinality(&self) -> Option<u128> {
            cardinality::any_sum(self.0.cardinality(), self.1.cardinality())
        }
    }

    impl<T: Generate, E: Generate> Generate for Result<T, E> {
        type Item = Result<T::Item, E::Item>;
        type Shrink = Shrinker<T::Shrink, E::Shrink>;

        const CARDINALITY: Option<u128> = cardinality::any_sum(T::CARDINALITY, E::CARDINALITY);

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrinker(match self {
                Ok(generator) => Ok(generator.generate(state)),
                Err(generator) => Err(generator.generate(state)),
            })
        }

        fn cardinality(&self) -> Option<u128> {
            match self {
                Ok(generator) => generator.cardinality(),
                Err(generator) => generator.cardinality(),
            }
        }
    }

    impl<T: Shrink, E: Shrink> Shrink for Shrinker<T, E> {
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

macro_rules! pointer {
    ($m: ident, $t: ident) => {
        mod $m {
            use super::*;

            impl<G: FullGenerate + ?Sized> FullGenerate for $t<G> {
                type Generator = Convert<G::Generator, Self::Item>;
                type Item = $t<G::Item>;

                fn generator() -> Self::Generator {
                    Convert(PhantomData, G::generator())
                }
            }

            impl<G: Generate + ?Sized> Generate for $t<G> {
                type Item = G::Item;
                type Shrink = G::Shrink;

                const CARDINALITY: Option<u128> = G::CARDINALITY;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    G::generate(self, state)
                }

                fn cardinality(&self) -> Option<u128> {
                    G::cardinality(self)
                }
            }
        }
    };
}

pointer!(boxed, Box);
pointer!(rc, Rc);
pointer!(arc, Arc);
