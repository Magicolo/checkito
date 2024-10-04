use crate::{
    generate::{FullGenerator, Generator, IntoGenerator, State},
    shrink::Shrinker,
    utility::tuples,
};
use core::f64;

#[repr(transparent)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Any<T: ?Sized>(pub T);

#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Weight<T: ?Sized> {
    weight: f64,
    value: T,
}

impl<T> Weight<T> {
    pub fn new(weight: f64, value: T) -> Self {
        assert!(weight.is_finite());
        assert!(weight > f64::EPSILON);
        Self { weight, value }
    }

    pub fn weight(&self) -> f64 {
        self.weight
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn into_value(self) -> T {
        self.value
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
            .map(|Weight { weight, .. }| weight.max(f64::EPSILON))
            .sum::<f64>();
        assert!(total.is_finite());
        let mut random = state.random().f64() * total;
        for Weight { weight, value } in items {
            let weight = weight.max(f64::EPSILON);
            if random < weight {
                return Some(value);
            } else {
                random -= weight;
            }
        }
        unreachable!("there is at least one item in the slice and weights are finite and `> 0.0`");
    }
}

impl<T: ?Sized> AsRef<T> for Any<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<G: FullGenerator + ?Sized> FullGenerator for Any<G>
where
    Any<G::FullGen>: Generator,
{
    type FullGen = Any<G::FullGen>;
    type Item = <Any<G::FullGen> as Generator>::Item;

    fn full_gen() -> Self::FullGen {
        Any(G::full_gen())
    }
}

impl<G: IntoGenerator> IntoGenerator for Any<G>
where
    Any<G::IntoGen>: Generator,
{
    type IntoGen = Any<G::IntoGen>;
    type Item = <Any<G::IntoGen> as Generator>::Item;

    fn into_gen(self) -> Self::IntoGen {
        Any(self.0.into_gen())
    }
}

const fn as_slice<T>(slice: &[T]) -> &[T] {
    slice
}

macro_rules! collection {
    ($t:ty, $i:ident, [$($n:ident)?]) => {
        impl<T: Generator $(,const $n: usize)?> Generator for $t {
            type Item = Option<T::Item>;
            type Shrink = Option<T::Shrink>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Some($i(as_slice(self.as_ref()), state)?.generate(state))
            }

            fn constant(&self) -> bool {
                as_slice(self.as_ref()).len() <= 1
            }
        }
    };
}

collection!(Any<[T]>, indexed, []);
collection!(Any<&[T]>, indexed, []);
collection!(Any<&mut [T]>, indexed, []);
collection!([Weight<T>], weighted, []);
collection!(Any<[T; N]>, indexed, [N]);
collection!([Weight<T>; N], weighted, [N]);
collection!(Any<Box<[T]>>, indexed, []);
collection!(Box<[Weight<T>]>, weighted, []);
collection!(Any<Vec<T>>, indexed, []);
collection!(Vec<Weight<T>>, weighted, []);

macro_rules! tuple {
    ($n:ident, $c:tt) => {};
    ($n:ident, $c:tt $(, $ps:ident, $ts:ident, $is:tt)+) => {
        impl<$($ts: Generator,)*> Generator for orn::$n::Or<$($ts,)*> {
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

        impl<$($ts: Shrinker,)*> Shrinker for orn::$n::Or<$($ts,)*> {
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

        impl<$($ts: Generator,)*> Generator for Any<($($ts,)*)> {
            type Item = orn::$n::Or<$($ts::Item,)*>;
            type Shrink = orn::$n::Or<$($ts::Shrink,)*>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                match state.random().u8(..$c) {
                    $($is => orn::$n::Or::$ts(self.0.$is.generate(state)),)*
                    _ => unreachable!(),
                }
            }

            fn constant(&self) -> bool {
                $c <= 1
            }
        }

        impl<$($ts: Generator,)*> Generator for ($(Weight<$ts>,)*) {
            type Item = orn::$n::Or<$($ts::Item,)*>;
            type Shrink = orn::$n::Or<$($ts::Shrink,)*>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                let _total = $(self.$is.weight.max(f64::EPSILON) +)* 0.0;
                assert!(_total.is_finite());
                let mut _weight = state.random().f64() * _total;
                $(
                    let Weight { weight, value } = &self.$is;
                    let weight = weight.max(f64::EPSILON);
                    if _weight < weight {
                        return orn::$n::Or::$ts(value.generate(state));
                    } else {
                        _weight -= weight;
                    }
                )*
                unreachable!("there is at least one item in the tuple and weights are finite and `> 0.0`");
            }

            fn constant(&self) -> bool {
                $c <= 1
            }
        }
    };
}

tuples!(tuple);
