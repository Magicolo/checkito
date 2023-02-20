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
    weight: f64,
    value: T,
}

fn indexed<'a, T>(items: &'a [T], state: &mut State) -> Option<&'a T> {
    if items.len() == 0 {
        None
    } else {
        Some(&items[state.random.usize(0..items.len())])
    }
}

fn weighted<'a, T>(items: &'a [Weight<T>], state: &mut State) -> Option<&'a T> {
    let total = items.iter().map(|weight| weight.weight).sum::<f64>();
    let mut random = state.random.f64() * total;
    for weight in items {
        if random < weight.weight {
            return Some(&weight.value);
        } else {
            random -= weight.weight;
        }
    }
    None
}

impl<T> Weight<T> {
    #[inline]
    pub const fn new(value: T, weight: f64) -> Self {
        Self { value, weight }
    }
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
collection!(Vec<T>, indexed, []);
collection!(Vec<Weight<T>>, weighted, []);

macro_rules! tuple {
    ($n:ident, $c:tt) => {};
    ($n:ident, $c:tt, $p:ident, $t:ident, $i:tt $(, $ps:ident, $ts:ident, $is:tt)*) => {
        pub(crate) mod $n {
            use super::*;

            #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
            pub enum One<$t, $($ts = $t,)*> {
                $t($t),
                $($ts($ts),)*
            }

            impl<$t: Generate, $($ts: Generate<Item = $t::Item>,)*> Generate for One<$t, $($ts,)*> {
                type Item = $t::Item;
                type Shrink = One<$t::Shrink, $($ts::Shrink,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    match self {
                        Self::$t(generate) => { let (item, shrink) = generate.generate(state); (item, One::$t(shrink)) },
                        $(Self::$ts(generate) => { let (item, shrink) = generate.generate(state); (item, One::$ts(shrink)) },)*
                    }
                }
            }

            impl<$t: Shrink, $($ts: Shrink<Item = $t::Item>,)*> Shrink for One<$t, $($ts,)*> {
                type Item = $t::Item;

                fn generate(&self) -> Self::Item {
                    match self {
                        One::$t(shrink) => shrink.generate(),
                        $(One::$ts(shrink) => shrink.generate(),)*
                    }
                }

                fn shrink(&mut self) -> Option<Self> {
                    match self {
                        Self::$t(shrink) => Some(Self::$t(shrink.shrink()?)),
                        $(Self::$ts(shrink) => Some(Self::$ts(shrink.shrink()?)),)*
                    }
                }
            }

            impl<$t: Generate, $($ts: Generate<Item = $t::Item>,)*> Generate for Any<($t, $($ts,)*)> {
                type Item = $t::Item;
                type Shrink = One<$t::Shrink, $($ts::Shrink,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    let ($p, $($ps,)*) = &self.0;
                    let count = count!($p $(,$ps)*);
                    match state.random.u8(..count) {
                        $i => { let (item, shrink) = $p.generate(state); (item, One::$t(shrink)) }
                        $($is => { let (item, shrink) = $ps.generate(state); (item, One::$ts(shrink)) })*
                        _ => unreachable!(),
                    }
                }
            }

            impl<$t: Generate, $($ts: Generate<Item = $t::Item>,)*> Generate for Any<(Weight<$t>, $(Weight<$ts>,)*)> {
                type Item = $t::Item;
                type Shrink = One<$t::Shrink, $($ts::Shrink,)*>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    let ($p, $($ps,)*) = &self.0;
                    let total = $p.weight $(+ $ps.weight)*;
                    let mut _weight = state.random.f64() * total;
                    let mut _index = 0;

                    if _weight < $p.weight {
                        let (item, shrink) = $p.value.generate(state);
                        return (item, One::$t(shrink));
                    } else {
                        _weight -= $p.weight;
                    }

                    $(
                        _index += 1;
                        if _weight < $ps.weight {
                            let (item, shrink) = $ps.value.generate(state);
                            return (item, One::$ts(shrink));
                        } else {
                            _weight -= $ps.weight;
                        }
                    )*

                    unreachable!();
                }
            }
        }
    };
}

tuples!(tuple);
