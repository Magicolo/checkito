use crate::{
    generate::{FullGenerate, Generate, IntoGenerate, State},
    shrink::Shrink,
    tuples,
};

#[repr(transparent)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Any<T: ?Sized>(pub T);

#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Weight<W, T: ?Sized> {
    pub weight: W,
    pub value: T,
}

pub trait Unify<T> {
    fn unify(self) -> T;
}

impl<W: Generate<Item = f64>, T> Weight<W, T> {
    pub const fn new(weight: W, value: T) -> Self {
        Self { weight, value }
    }
}

fn indexed<'a, T>(items: &'a [T], state: &mut State) -> Option<&'a T> {
    if items.len() == 0 {
        None
    } else {
        items.get(state.random().usize(0..items.len()))
    }
}

fn weighted<'a, W: Generate<Item = f64>, T>(
    items: &'a [Weight<W, T>],
    state: &mut State,
) -> Option<&'a T> {
    let weights = items
        .iter()
        .map(|weight| (weight.weight.generate(state).item().max(0.0), &weight.value))
        .collect::<Vec<_>>();
    let total = weights.iter().map(|pair| pair.0).sum::<f64>();
    let mut random = state.random().f64() * total;
    for (weight, value) in weights {
        if random < weight {
            return Some(value);
        } else {
            random -= weight;
        }
    }
    None
}

impl<T: ?Sized> AsRef<T> for Any<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<G: FullGenerate + ?Sized> FullGenerate for Any<G>
where
    Any<G::Generate>: Generate,
{
    type Item = <Any<G::Generate> as Generate>::Item;
    type Generate = Any<G::Generate>;

    fn generator() -> Self::Generate {
        Any(G::generator())
    }
}

impl<G: IntoGenerate> IntoGenerate for Any<G>
where
    Any<G::Generate>: Generate,
{
    type Item = <Any<G::Generate> as Generate>::Item;
    type Generate = Any<G::Generate>;

    fn generator(self) -> Self::Generate {
        Any(self.0.generator())
    }
}

impl<G: ?Sized> Generate for Any<&G>
where
    Any<G>: Generate,
{
    type Item = <Any<G> as Generate>::Item;
    type Shrink = <Any<G> as Generate>::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        unsafe { &*(self.0 as *const G as *const Any<G>) }.generate(state)
    }
}

impl<G: ?Sized> Generate for Any<&mut G>
where
    Any<G>: Generate,
{
    type Item = <Any<G> as Generate>::Item;
    type Shrink = <Any<G> as Generate>::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Any(&*self.0).generate(state)
    }
}

macro_rules! collection {
    ($t:ty, $i:ident, [$($w:ident)?], [$($n:ident)?]) => {
        impl<T: Generate $(,$w: Generate<Item = f64>)? $(,const $n: usize)?> Generate for $t {
            type Item = Option<T::Item>;
            type Shrink = Option<T::Shrink>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Some($i(self.as_ref(), state)?.generate(state))
            }
        }
    };
}

collection!(Any<[T]>, indexed, [], []);
collection!([Weight<W, T>], weighted, [W], []);
collection!(Any<[T; N]>, indexed, [], [N]);
collection!([Weight<W, T>; N], weighted, [W], [N]);
collection!(Any<Box<[T]>>, indexed, [], []);
collection!(Box<[Weight<W, T>]>, weighted, [W], []);
collection!(Any<Vec<T>>, indexed, [], []);
collection!(Vec<Weight<W, T>>, weighted, [W], []);

macro_rules! tuple {
    ($n:ident, $c:tt) => {};
    ($n:ident, $c:tt $(, $ps:ident, $ts:ident, $is:tt)+) => {
        pub mod $n {
            use super::*;

            #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
            pub enum One<$($ts,)*> {
                $($ts($ts),)*
            }

            impl<$($ts,)*> One<$($ts,)*> {
                pub const fn as_ref(&self) -> One<$(&$ts,)*> {
                    match self {
                        $(Self::$ts(item) => One::$ts(item),)*
                    }
                }

                pub fn as_mut(&mut self) -> One<$(&mut $ts,)*> {
                    match self {
                        $(Self::$ts(item) => One::$ts(item),)*
                    }
                }
            }

            impl<T, $($ts: Into<T>,)*> Unify<T> for One<$($ts,)*> {
                fn unify(self) -> T {
                    match self {
                        $(Self::$ts(item) => item.into(),)*
                    }
                }
            }

            impl<$($ts: Generate,)*> Generate for One<$($ts,)*> {
                type Item = One<$($ts::Item,)*>;
                type Shrink = One<$($ts::Shrink,)*>;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    match self {
                        $(Self::$ts(generate) => One::$ts(generate.generate(state)),)*
                    }
                }
            }

            impl<$($ts: Shrink,)*> Shrink for One<$($ts,)*> {
                type Item = One<$($ts::Item,)*>;

                fn item(&self) -> Self::Item {
                    match self {
                        $(One::$ts(shrink) => One::$ts(shrink.item()),)*
                    }
                }

                fn shrink(&mut self) -> Option<Self> {
                    match self {
                        $(Self::$ts(shrink) => Some(Self::$ts(shrink.shrink()?)),)*
                    }
                }
            }

            impl<$($ts: Generate,)*> Generate for Any<($($ts,)*)> {
                type Item = One<$($ts::Item,)*>;
                type Shrink = One<$($ts::Shrink,)*>;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    match state.random().u8(..$c) {
                        $($is => One::$ts(self.0.$is.generate(state)),)*
                        _ => unreachable!(),
                    }
                }
            }

            #[allow(non_camel_case_types)]
            impl<$($ps: Generate<Item = f64>, $ts: Generate,)*> Generate for ($(Weight<$ps, $ts>,)*) {
                type Item = One<$($ts::Item,)*>;
                type Shrink = One<$($ts::Shrink,)*>;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    let weights = ($(self.$is.weight.generate(state).item().max(0.0),)*);
                    let total = $(weights.$is +)* 0.0;
                    let mut _weight = state.random().f64() * total;
                    $(
                        if _weight < weights.$is {
                            return One::$ts(self.$is.value.generate(state));
                        } else {
                            _weight -= weights.$is;
                        }
                    )*
                    unreachable!();
                }
            }
        }
    };
}

tuples!(tuple);
