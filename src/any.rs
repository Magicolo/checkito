use crate::{
    count,
    generate::{FullGenerate, Generate, IntoGenerate, State},
    shrink::Shrink,
    tuples,
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Any<T: ?Sized>(pub T);
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Weight<T: ?Sized> {
    pub weight: f64,
    pub value: T,
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

impl<G: FullGenerate> FullGenerate for Any<G>
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

macro_rules! collection {
    ($t:ty, $i:ident, [$($n:ident)?]) => {
        impl<T: Generate $(,const $n: usize)?> Generate for Any<$t> {
            type Item = Option<T::Item>;
            type Shrink = Option<T::Shrink>;

            fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                match $i(self.0.as_ref(), state) {
                    Some(generate) => {
                        let (item, shrink) = generate.generate(state);
                        (Some(item), Some(shrink))
                    }
                    None => (None, None)
                }
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
    ($n:ident, $c:tt, $p:ident, $t:ident, $i:tt $(, $ps:ident, $ts:ident, $is:tt)*) => {
        pub mod $n {
            use super::*;

            #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
            pub enum One<$t, $($ts = $t,)*> {
                $t($t),
                $($ts($ts),)*
            }

            impl<$t, $($ts,)*> One<$t, $($ts,)*> {
                pub const fn as_ref(&self) -> One<&$t, $(&$ts,)*> {
                    match self {
                        Self::$t(item) => One::$t(item),
                        $(Self::$ts(item) => One::$ts(item),)*
                    }
                }

                pub fn as_mut(&mut self) -> One<&mut $t, $(&mut $ts,)*> {
                    match self {
                        Self::$t(item) => One::$t(item),
                        $(Self::$ts(item) => One::$ts(item),)*
                    }
                }
            }

            impl<$t, $($ts: Into<$t>,)*> One<$t, $($ts,)*> {
                pub fn fuse(self) -> $t {
                    match self {
                        Self::$t(item) => item,
                        $(Self::$ts(item) => item.into(),)*
                    }
                }
            }

            impl<$t: Generate, $($ts: Generate,)*> Generate for One<$t, $($ts,)*> {
                type Item = One<$t::Item, $($ts::Item,)*>;
                type Shrink = One<$t::Shrink, $($ts::Shrink,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    match self {
                        Self::$t(generate) => { let (item, shrink) = generate.generate(state); (One::$t(item), One::$t(shrink)) },
                        $(Self::$ts(generate) => { let (item, shrink) = generate.generate(state); (One::$ts(item), One::$ts(shrink)) },)*
                    }
                }
            }

            impl<$t: Shrink, $($ts: Shrink,)*> Shrink for One<$t, $($ts,)*> {
                type Item = One<$t::Item, $($ts::Item,)*>;

                fn generate(&self) -> Self::Item {
                    match self {
                        One::$t(shrink) => One::$t(shrink.generate()),
                        $(One::$ts(shrink) => One::$ts(shrink.generate()),)*
                    }
                }

                fn shrink(&mut self) -> Option<Self> {
                    match self {
                        Self::$t(shrink) => Some(Self::$t(shrink.shrink()?)),
                        $(Self::$ts(shrink) => Some(Self::$ts(shrink.shrink()?)),)*
                    }
                }
            }

            impl<$t: Generate, $($ts: Generate,)*> Generate for Any<($t, $($ts,)*)> {
                type Item = One<$t::Item, $($ts::Item,)*>;
                type Shrink = One<$t::Shrink, $($ts::Shrink,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    const COUNT: u8 = count!($t $(,$ts)*);
                    match state.random().u8(..COUNT) {
                        $i => { let (item, shrink) = self.0.$i.generate(state); (One::$t(item), One::$t(shrink)) }
                        $($is => { let (item, shrink) = self.0.$is.generate(state); (One::$ts(item), One::$ts(shrink)) })*
                        _ => unreachable!(),
                    }
                }
            }

            impl<$t: Generate, $($ts: Generate,)*> Generate for Any<(Weight<$t>, $(Weight<$ts>,)*)> {
                type Item = One<$t::Item, $($ts::Item,)*>;
                type Shrink = One<$t::Shrink, $($ts::Shrink,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    let total = self.0.$i.weight $(+ self.0.$is.weight)*;
                    let mut _weight = state.random().f64() * total;
                    let mut _index = 0;

                    if _weight < self.0.$i.weight {
                        let (item, shrink) = self.0.$i.value.generate(state);
                        return (One::$t(item), One::$t(shrink));
                    } else {
                        _weight -= self.0.$i.weight;
                    }

                    $(
                        _index += 1;
                        if _weight < self.0.$is.weight {
                            let (item, shrink) = self.0.$is.value.generate(state);
                            return (One::$ts(item), One::$ts(shrink));
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
