use crate::{
    generate::{Generate, State},
    shrink::Shrink,
    utility::tuples,
};
use core::f64;
use ref_cast::RefCast;

#[repr(transparent)]
#[derive(Clone, Debug, RefCast)]
pub struct Any<G: ?Sized>(pub(crate) G);

#[derive(Clone, Debug)]
pub struct Shrinker<S>(pub(crate) Option<S>);

#[derive(Clone, Debug)]
pub struct Weight<T: ?Sized> {
    weight: f64,
    generator: T,
}

impl<T> Weight<T> {
    pub const fn weight(&self) -> f64 {
        self.weight
    }

    pub const fn value(&self) -> &T {
        &self.generator
    }
}

impl<G: Generate> Weight<G> {
    pub fn new(weight: f64, generator: G) -> Self {
        assert!(weight.is_finite());
        assert!(weight > f64::EPSILON);
        Self { weight, generator }
    }
}

impl<G: Generate + ?Sized> Weight<G> {
    fn constant(&self) -> bool {
        self.generator.constant()
    }
}

fn indexed<'a, T>(items: &'a [T], state: &mut State) -> Option<&'a T> {
    if items.is_empty() {
        None
    } else {
        items.get(state.random().usize(0..items.len()))
    }
}

fn weighted<'a, T>(items: &'a [Weight<T>], state: &mut State) -> Option<&'a T> {
    if items.is_empty() {
        None
    } else {
        let total = items
            .iter()
            .map(|Weight { weight, .. }| weight)
            .sum::<f64>();
        assert!(total.is_finite());
        let mut random = state.random().f64() * total;
        for Weight {
            weight,
            generator: value,
        } in items
        {
            if random < *weight {
                return Some(value);
            } else {
                random -= weight;
            }
        }
        unreachable!("there is at least one item in the slice and weights are finite and `> 0.0`");
    }
}

impl<G: ?Sized> Generate for Any<Any<G>>
where
    Any<G>: Generate,
{
    type Item = <Any<G> as Generate>::Item;
    type Shrink = <Any<G> as Generate>::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Any::ref_cast(&self.0.0).generate(state)
    }

    fn constant(&self) -> bool {
        Any::ref_cast(&self.0.0).constant()
    }
}

impl<G: ?Sized> Generate for Any<&G>
where
    Any<G>: Generate,
{
    type Item = <Any<G> as Generate>::Item;
    type Shrink = <Any<G> as Generate>::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Any::ref_cast(self.0).generate(state)
    }

    fn constant(&self) -> bool {
        Any::ref_cast(self.0).constant()
    }
}

impl<G: ?Sized> Generate for Any<&mut G>
where
    Any<G>: Generate,
{
    type Item = <Any<G> as Generate>::Item;
    type Shrink = <Any<G> as Generate>::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Any::ref_cast(self.0).generate(state)
    }

    fn constant(&self) -> bool {
        Any::ref_cast(self.0).constant()
    }
}

macro_rules! slice {
    ($t: ty, $i: ident, [$($n: ident)?]) => {
        impl<G: Generate $(,const $n: usize)?> Generate for Any<$t> {
            type Item = Option<G::Item>;
            type Shrink = Shrinker<G::Shrink>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Shrinker($i(&self.0, state).map(|generator| generator.generate(state)))
            }

            fn constant(&self) -> bool {
                self.0.iter().all(|generator| generator.constant())
            }
        }
    };
}

slice!([G], indexed, []);
slice!([G; N], indexed, [N]);
slice!(Vec<G>, indexed, []);
slice!(Box<[G]>, indexed, []);
slice!([Weight<G>], weighted, []);
slice!([Weight<G>; N], weighted, [N]);
slice!(Vec<Weight<G>>, weighted, []);
slice!(Box<[Weight<G>]>, weighted, []);

impl<S: Shrink> Shrink for Shrinker<S> {
    type Item = Option<S::Item>;

    fn item(&self) -> Self::Item {
        Some(self.0.as_ref()?.item())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self(self.0.as_mut()?.shrink()))
    }
}

macro_rules! tuple {
    ($n:ident, $c:tt) => {};
    ($n:ident, $c:tt $(, $ps:ident, $ts:ident, $is:tt)+) => {
        impl<$($ts: Generate,)*> Generate for orn::$n::Or<$($ts,)*> {
            type Item = orn::$n::Or<$($ts::Item,)*>;
            type Shrink = orn::$n::Or<$($ts::Shrink,)*>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                match self {
                    $(Self::$ts(generator) => orn::$n::Or::$ts(generator.generate(state)),)*
                }
            }

            fn constant(&self) -> bool {
                match self {
                    $(Self::$ts(generator) => generator.constant(),)*
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

            fn generate(&self, state: &mut State) -> Self::Shrink {
                match state.random().u8(..$c) {
                    $($is => orn::$n::Or::$ts(self.0.$is.generate(state)),)*
                    _ => unreachable!(),
                }
            }

            fn constant(&self) -> bool {
                $(self.0.$is.constant() &&)* true
            }
        }

        impl<$($ts: Generate,)*> Generate for ($(Weight<$ts>,)*) {
            type Item = orn::$n::Or<$($ts::Item,)*>;
            type Shrink = orn::$n::Or<$($ts::Shrink,)*>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                let _total = $(self.$is.weight +)* 0.0;
                assert!(_total.is_finite());
                let mut _weight = state.random().f64() * _total;
                $(
                    let Weight { weight, generator } = &self.$is;
                    if _weight < *weight {
                        return orn::$n::Or::$ts(generator.generate(state));
                    } else {
                        _weight -= weight;
                    }
                )*
                unreachable!("there is at least one item in the tuple and weights are finite and `> 0.0`");
            }

            fn constant(&self) -> bool {
                $(self.$is.constant() &&)* true
            }
        }
    };
}

tuples!(tuple);
