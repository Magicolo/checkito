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

pub mod character {
    use super::*;
    use crate::{any::Any, unify::Unify};
    use core::ops::RangeInclusive;

    /// A generator for ASCII letters (`a-z`, `A-Z`).
    #[derive(Clone, Debug)]
    pub struct Letter(Unify<Any<(RangeInclusive<char>, RangeInclusive<char>)>, char>);

    /// A generator for ASCII digits (`0-9`).
    #[derive(Clone, Debug)]
    pub struct Digit(RangeInclusive<char>);

    /// A generator for all ASCII characters (0-127).
    #[derive(Clone, Debug)]
    pub struct Ascii(RangeInclusive<char>);

    impl Letter {
        pub const fn new() -> Self {
            Self(crate::prelude::unify(crate::prelude::any((
                'a'..='z',
                'A'..='Z',
            ))))
        }
    }

    impl Default for Letter {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Generate for Letter {
        type Item = char;
        type Shrink = <Unify<Any<(RangeInclusive<char>, RangeInclusive<char>)>, char> as Generate>::Shrink;

        const CARDINALITY: Option<u128> = <Unify<Any<(RangeInclusive<char>, RangeInclusive<char>)>, char> as Generate>::CARDINALITY;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            self.0.generate(state)
        }

        fn cardinality(&self) -> Option<u128> {
            self.0.cardinality()
        }
    }

    impl Digit {
        pub const fn new() -> Self {
            Self('0'..='9')
        }
    }

    impl Default for Digit {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Generate for Digit {
        type Item = char;
        type Shrink = <RangeInclusive<char> as Generate>::Shrink;

        const CARDINALITY: Option<u128> = <RangeInclusive<char> as Generate>::CARDINALITY;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            self.0.generate(state)
        }

        fn cardinality(&self) -> Option<u128> {
            self.0.cardinality()
        }
    }

    impl Ascii {
        pub const fn new() -> Self {
            Self(0 as char..=127 as char)
        }
    }

    impl Default for Ascii {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Generate for Ascii {
        type Item = char;
        type Shrink = <RangeInclusive<char> as Generate>::Shrink;

        const CARDINALITY: Option<u128> = <RangeInclusive<char> as Generate>::CARDINALITY;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            self.0.generate(state)
        }

        fn cardinality(&self) -> Option<u128> {
            self.0.cardinality()
        }
    }
}

pub mod number {
    use super::*;
    use crate::primitive::Number as NumberTrait;

    /// A generator for the full range of any [`Number`](crate::primitive::Number) type.
    #[derive(Clone, Debug)]
    pub struct Number<T: NumberTrait>(pub(crate) T::Full);

    /// A generator for any non-negative [`Number`](crate::primitive::Number) type (includes `0`).
    #[derive(Clone, Debug)]
    pub struct Positive<T: NumberTrait>(pub(crate) T::Positive);

    /// A generator for any non-positive [`Number`](crate::primitive::Number) type (includes `0`).
    #[derive(Clone, Debug)]
    pub struct Negative<T: NumberTrait>(pub(crate) T::Negative);

    impl<T: NumberTrait> Number<T> {
        pub const fn new() -> Self {
            Self(T::FULL)
        }
    }

    impl<T: NumberTrait> Default for Number<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<T: NumberTrait> Generate for Number<T> {
        type Item = T;
        type Shrink = <T::Full as Generate>::Shrink;

        const CARDINALITY: Option<u128> = <T::Full as Generate>::CARDINALITY;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            self.0.generate(state)
        }

        fn cardinality(&self) -> Option<u128> {
            self.0.cardinality()
        }
    }

    impl<T: NumberTrait> Positive<T> {
        pub const fn new() -> Self {
            Self(T::POSITIVE)
        }
    }

    impl<T: NumberTrait> Default for Positive<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<T: NumberTrait> Generate for Positive<T> {
        type Item = T;
        type Shrink = <T::Positive as Generate>::Shrink;

        const CARDINALITY: Option<u128> = <T::Positive as Generate>::CARDINALITY;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            self.0.generate(state)
        }

        fn cardinality(&self) -> Option<u128> {
            self.0.cardinality()
        }
    }

    impl<T: NumberTrait> Negative<T> {
        pub const fn new() -> Self {
            Self(T::NEGATIVE)
        }
    }

    impl<T: NumberTrait> Default for Negative<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<T: NumberTrait> Generate for Negative<T> {
        type Item = T;
        type Shrink = <T::Negative as Generate>::Shrink;

        const CARDINALITY: Option<u128> = <T::Negative as Generate>::CARDINALITY;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            self.0.generate(state)
        }

        fn cardinality(&self) -> Option<u128> {
            self.0.cardinality()
        }
    }
}

pub mod with {
    use super::*;

    /// A generator created from a closure that produces a value.
    #[derive(Debug, Clone)]
    pub struct With<T, F: Fn() -> T + Clone>(pub(crate) PhantomData<T>, pub(crate) F);

    impl<T, F: Fn() -> T + Clone> With<T, F> {
        pub const fn new(generator: F) -> Self {
            Self(PhantomData, generator)
        }
    }

    #[derive(Debug, Clone)]
    pub struct Shrinker<F>(pub(crate) F);

    impl<T, F: Fn() -> T + Clone> Generate for With<T, F> {
        type Item = T;
        type Shrink = Shrinker<F>;

        const CARDINALITY: Option<u128> = Some(1);

        fn generate(&self, _state: &mut State) -> Self::Shrink {
            Shrinker(self.1.clone())
        }

        fn cardinality(&self) -> Option<u128> {
            Some(1)
        }
    }

    impl<T, F: Fn() -> T + Clone> Shrink for Shrinker<F> {
        type Item = T;

        fn item(&self) -> Self::Item {
            (self.0)()
        }

        fn shrink(&mut self) -> Option<Self> {
            None
        }
    }
}
