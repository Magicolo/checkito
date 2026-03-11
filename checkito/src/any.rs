use crate::{
    cardinality,
    generate::Generate,
    primitive::Constant,
    shrink::Shrink,
    state::{State, Weight},
    utility::tuples,
};
use ref_cast::RefCast;
use std::{rc::Rc, sync::Arc};

#[repr(transparent)]
#[derive(Clone, Debug, RefCast)]
pub struct Any<G: ?Sized>(pub(crate) G);

#[derive(Clone, Debug)]
pub struct Shrinker<S>(pub(crate) Option<S>);

impl<T: ?Sized, U: AsRef<T> + ?Sized> AsRef<T> for Any<U> {
    fn as_ref(&self) -> &T {
        self.0.as_ref()
    }
}

/// Implement Generate for reference types (&G, &mut G) that delegate to Any<G>
macro_rules! reference {
    ($($type:ty),*) => {
        $(
            impl<G: ?Sized> Generate for Any<$type>
            where
                Any<G>: Generate,
            {
                type Item = <Any<G> as Generate>::Item;
                type Shrink = <Any<G> as Generate>::Shrink;

                const CARDINALITY: Option<u128> = Any::<G>::CARDINALITY;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    Any::ref_cast(self.0).generate(state)
                }

                fn cardinality(&self) -> Option<u128> {
                    Any::ref_cast(self.0).cardinality()
                }
            }
        )*
    };
}

reference!(&G, &mut G);

impl<S: Shrink> Shrink for Shrinker<S> {
    type Item = Option<S::Item>;

    fn item(&self) -> Self::Item {
        Some(self.0.as_ref()?.item())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self(self.0.as_mut()?.shrink()))
    }
}

impl<C: Constant> Constant for Any<C> {
    const VALUE: Self = Self(C::VALUE);
}

macro_rules! pointer {
    ($t: ident) => {
        impl<G: ?Sized> Generate for Any<$t<G>>
        where
            Any<G>: Generate,
        {
            type Item = <Any<G> as Generate>::Item;
            type Shrink = <Any<G> as Generate>::Shrink;

            const CARDINALITY: Option<u128> = Any::<G>::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Any::ref_cast(self.0.as_ref()).generate(state)
            }

            fn cardinality(&self) -> Option<u128> {
                Any::ref_cast(self.0.as_ref()).cardinality()
            }
        }
    };
}

pointer!(Box);
pointer!(Rc);
pointer!(Arc);

macro_rules! slice {
    ($t: ty, $i: ident, $r: ident, [$($n: ident)?]) => {
        impl<G: Generate $(, const $n: usize)?> Generate for $t {
            type Item = Option<G::Item>;
            type Shrink = Shrinker<G::Shrink>;

            slice!(STATIC, G $(, $n)?);

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Shrinker(state.$i($r(self.as_ref())).map(|generator: &G| generator.generate(state)))
            }

            fn cardinality(&self) -> Option<u128> {
                $r(self.as_ref())
                    .map(|generator| generator.cardinality())
                    .fold(Some(0), cardinality::any_sum)
            }
        }
    };
    (STATIC, $g: ident) => {
        const CARDINALITY: Option<u128> = None;
    };
    (STATIC, $g: ident, $n: ident) => {
        const CARDINALITY: Option<u128> = cardinality::any_repeat_static::<$n>($g::CARDINALITY);
    };
}

fn as_self<T>(slice: &[T]) -> impl ExactSizeIterator<Item = &T> + Clone {
    slice.iter()
}

fn as_ref<T>(slice: &[Weight<T>]) -> impl ExactSizeIterator<Item = Weight<&T>> + Clone {
    slice.iter().map(Weight::as_ref)
}

slice!(Any<[G]>, any_uniform, as_self, []);
slice!(Any<[Weight<G>]>, any_weighted, as_ref, []);
slice!(Any<[G; N]>, any_uniform, as_self, [N]);
slice!(Any<[Weight<G>; N]>, any_weighted, as_ref, [N]);
slice!(Any<Vec<G>>, any_uniform, as_self, []);
slice!(Any<Vec<Weight<G>>>, any_weighted, as_ref, []);

macro_rules! tuple {
    ($n:ident, $c:tt) => {};
    ($n:ident, $c:tt $(, $ps:ident, $ts:ident, $is:tt)*) => {
        impl<$($ts: Generate,)*> Generate for orn::$n::Or<$($ts,)*> {
            type Item = orn::$n::Or<$($ts::Item,)*>;
            type Shrink = orn::$n::Or<$($ts::Shrink,)*>;

            const CARDINALITY: Option<u128> = {
                let cardinality = Some(0);
                $(let cardinality = cardinality::any_sum(cardinality, $ts::CARDINALITY);)*
                cardinality
            };

            fn generate(&self, state: &mut State) -> Self::Shrink {
                match self {
                    $(Self::$ts(generator) => orn::$n::Or::$ts(generator.generate(state)),)*
                }
            }

            fn cardinality(&self) -> Option<u128> {
                match self {
                    $(Self::$ts(generator) => generator.cardinality(),)*
                }
            }
        }

        impl<$($ts: Shrink,)*> Shrink for orn::$n::Or<$($ts,)*> {
            type Item = orn::$n::Or<$($ts::Item,)*>;

            fn item(&self) -> Self::Item {
                match self {
                    $(orn::$n::Or::$ts(shrinker) => orn::$n::Or::$ts(shrinker.item()),)*
                }
            }

            fn shrink(&mut self) -> Option<Self> {
                match self {
                    $(Self::$ts(shrinker) => Some(Self::$ts(shrinker.shrink()?)),)*
                }
            }
        }

        impl<$($ts: Generate,)*> Generate for Any<($($ts,)*)> {
            type Item = orn::$n::Or<$($ts::Item,)*>;
            type Shrink = orn::$n::Or<$($ts::Shrink,)*>;

            const CARDINALITY: Option<u128> = {
                let cardinality = Some(0);
                $(let cardinality = cardinality::any_sum(cardinality, $ts::CARDINALITY);)*
                cardinality
            };

            fn generate(&self, state: &mut State) -> Self::Shrink {
                state.any_uniform([$(orn::$n::Or::$ts(&self.0.$is),)*]).unwrap().generate(state)
            }

            fn cardinality(&self) -> Option<u128> {
                let cardinality = Some(0);
                $(let cardinality = cardinality::any_sum(cardinality, self.0.$is.cardinality());)*
                cardinality
            }
        }

        impl<$($ts: Generate,)*> Generate for Any<($(Weight<$ts>,)*)> {
            type Item = orn::$n::Or<$($ts::Item,)*>;
            type Shrink = orn::$n::Or<$($ts::Shrink,)*>;

            const CARDINALITY: Option<u128> = {
                let cardinality = Some(0);
                $(let cardinality = cardinality::any_sum(cardinality, $ts::CARDINALITY);)*
                cardinality
            };

            fn generate(&self, state: &mut State) -> Self::Shrink {
                state.any_weighted([$(self.0.$is.as_ref().map(orn::$n::Or::$ts),)*]).unwrap().generate(state)
            }

            fn cardinality(&self) -> Option<u128> {
                let cardinality = Some(0);
                $(let cardinality = cardinality::any_sum(cardinality, self.0.$is.cardinality());)*
                cardinality
            }
        }
    };
}

tuples!(tuple);
