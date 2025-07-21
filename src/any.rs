use crate::{
    cardinality,
    generate::Generate,
    shrink::Shrink,
    state::{State, Weight},
    utility::tuples,
};
use core::f64;
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

impl<G: ?Sized> Generate for Any<&G>
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

impl<G: ?Sized> Generate for Any<&mut G>
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

impl<S: Shrink> Shrink for Shrinker<S> {
    type Item = Option<S::Item>;

    fn item(&self) -> Self::Item {
        Some(self.0.as_ref()?.item())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self(self.0.as_mut()?.shrink()))
    }
}

const fn as_slice<T>(slice: &[T]) -> &[T] {
    slice
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
    ($t: ty, $i: ident, [$($n: ident)?]) => {
        impl<G: Generate $(, const $n: usize)?> Generate for $t {
            type Item = Option<G::Item>;
            type Shrink = Shrinker<G::Shrink>;

            slice!(STATIC, G $(, $n)?);

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Shrinker(
                    state.$i(self.as_ref()).map(|generator| generator.generate(state)),
                )
            }

            fn cardinality(&self) -> Option<u128> {
                as_slice(self.as_ref())
                    .iter()
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

slice!(Any<[G]>, any_indexed, []);
slice!(Any<[G; N]>, any_indexed, [N]);
slice!(Any<Vec<G>>, any_indexed, []);
slice!([Weight<G>], any_weighted, []);
slice!([Weight<G>; N], any_weighted, [N]);
slice!(Vec<Weight<G>>, any_weighted, []);

macro_rules! tuple {
    ($n:ident, $c:tt) => {};
    ($n:ident, $c:tt $(, $ps:ident, $ts:ident, $is:tt)+) => {
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
                let value = state.with().size(1.0).u8(..$c);
                match value {
                    $($is => orn::$n::Or::$ts(self.0.$is.generate(state)),)*
                    _ => unreachable!(),
                }
            }

            fn cardinality(&self) -> Option<u128> {
                let cardinality = Some(0);
                $(let cardinality = cardinality::any_sum(cardinality, self.0.$is.cardinality());)*
                cardinality
            }
        }

        impl<$($ts: Generate,)*> Generate for ($(Weight<$ts>,)*) {
            type Item = orn::$n::Or<$($ts::Item,)*>;
            type Shrink = orn::$n::Or<$($ts::Shrink,)*>;

            const CARDINALITY: Option<u128> = {
                let cardinality = Some(0);
                $(let cardinality = cardinality::any_sum(cardinality, $ts::CARDINALITY);)*
                cardinality
            };

            fn generate(&self, state: &mut State) -> Self::Shrink {
                // TODO: Use `State::any_tuple`
                let _total = ($(self.$is.weight() +)* 0.0).min(f64::MAX);
                debug_assert!(_total > 0.0 && _total.is_finite());
                let mut _random = state.with().size(1.0).f64(0.0..=_total);
                debug_assert!(_random.is_finite());
                $(
                    let weight = self.$is.weight();
                    if _random < weight {
                        return orn::$n::Or::$ts(self.$is.value().generate(state));
                    } else {
                        _random -= weight;
                    }
                )*
                unreachable!("there is at least one item in the tuple and weights are finite and `> 0.0`");
            }

            fn cardinality(&self) -> Option<u128> {
                let cardinality = Some(0);
                $(let cardinality = cardinality::any_sum(cardinality, self.$is.cardinality());)*
                cardinality
            }
        }
    };
}

tuples!(tuple);
