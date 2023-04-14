use crate::{
    generate::{FullGenerate, Generate, IntoGenerate, State},
    shrink::Shrink,
    tuples,
};

#[repr(transparent)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Any<T: ?Sized>(pub T);

#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Weight<T: ?Sized> {
    pub weight: f64,
    pub value: T,
}

pub trait Fuse<T> {
    fn fuse(self) -> T;
}

impl<T> Weight<T> {
    pub const fn new(weight: f64, value: T) -> Self {
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

fn weighted<'a, T>(items: &'a [Weight<T>], state: &mut State) -> Option<&'a T> {
    let total = items.iter().map(|weight| weight.weight).sum::<f64>();
    let mut random = state.random().f64() * total;
    for weight in items {
        if random < weight.weight {
            return Some(&weight.value);
        } else {
            random -= weight.weight;
        }
    }
    None
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
    ($t:ty, $i:ident, [$($n:ident)?]) => {
        impl<T: Generate $(,const $n: usize)?> Generate for Any<$t> {
            type Item = Option<T::Item>;
            type Shrink = Option<T::Shrink>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Some($i(self.0.as_ref(), state)?.generate(state))
            }
        }
    };
}

collection!([T], indexed, []);
collection!([Weight<T>], weighted, []);
collection!([T; N], indexed, [N]);
collection!([Weight<T>; N], weighted, [N]);
collection!(Box<[T]>, indexed, []);
collection!(Box<[Weight<T>]>, weighted, []);
collection!(Vec<T>, indexed, []);
collection!(Vec<Weight<T>>, weighted, []);

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

            impl<T, $($ts: Into<T>,)*> Fuse<T> for One<$($ts,)*> {
                fn fuse(self) -> T {
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

            impl<$($ts: Generate,)*> Generate for Any<($(Weight<$ts>,)*)> {
                type Item = One<$($ts::Item,)*>;
                type Shrink = One<$($ts::Shrink,)*>;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    let total = $(self.0.$is.weight +)* 0.0;
                    let mut _weight = state.random().f64() * total;
                    $(
                        if _weight < self.0.$is.weight {
                            return One::$ts(self.0.$is.value.generate(state));
                        } else {
                            _weight -= self.0.$is.weight;
                        }
                    )*
                    unreachable!();
                }
            }
        }
    };
}

tuples!(tuple);
