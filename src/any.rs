use crate::{
    generate::{Generate, State},
    shrink::Shrink,
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
        assert!(weight >= f64::EPSILON);
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
            .sum::<f64>()
            .min(f64::MAX);
        debug_assert!(total > 0.0 && total.is_finite());
        let mut random = state.random().f64() * total;
        debug_assert!(random.is_finite());
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

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Any::ref_cast(self.0.as_ref()).generate(state)
            }

            fn constant(&self) -> bool {
                Any::ref_cast(self.0.as_ref()).constant()
            }
        }
    };
}

pointer!(Box);
pointer!(Rc);
pointer!(Arc);

macro_rules! slice {
    ($t: ty, $i: ident, [$($n: ident)?]) => {
        impl<G: Generate $(,const $n: usize)?> Generate for $t {
            type Item = Option<G::Item>;
            type Shrink = Shrinker<G::Shrink>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Shrinker($i(as_slice(self.as_ref()), state).map(|generator| generator.generate(state)))
            }

            fn constant(&self) -> bool {
                as_slice(self.as_ref()).iter().all(|generator| generator.constant())
            }
        }
    };
}

slice!(Any<[G]>, indexed, []);
slice!(Any<[G; N]>, indexed, [N]);
slice!(Any<Vec<G>>, indexed, []);
slice!([Weight<G>], weighted, []);
slice!([Weight<G>; N], weighted, [N]);
slice!(Vec<Weight<G>>, weighted, []);

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
                let _total = ($(self.$is.weight +)* 0.0).min(f64::MAX);
                debug_assert!(_total > 0.0 && _total.is_finite());
                let mut _random = state.random().f64() * _total;
                debug_assert!(_random.is_finite());
                $(
                    let Weight { weight, generator } = &self.$is;
                    if _random < *weight {
                        return orn::$n::Or::$ts(generator.generate(state));
                    } else {
                        _random -= weight;
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
