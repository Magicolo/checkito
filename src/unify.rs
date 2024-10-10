use crate::{
    generate::{Generate, State},
    shrink::Shrink,
    utility::tuples,
};
use core::marker::PhantomData;

#[derive(Debug)]
pub struct Unify<T: ?Sized, I: ?Sized>(pub(crate) PhantomData<I>, pub(crate) T);

impl<T: Clone, I: ?Sized> Clone for Unify<T, I> {
    fn clone(&self) -> Self {
        Self(PhantomData, self.1.clone())
    }
}

impl<G: Generate + ?Sized, I> Generate for Unify<G, I>
where
    Unify<G::Shrink, I>: Shrink<Item = I>,
{
    type Item = I;
    type Shrink = Unify<G::Shrink, I>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Unify(PhantomData, self.1.generate(state))
    }

    fn constant(&self) -> bool {
        self.1.constant()
    }
}

macro_rules! tuple {
    ($n:ident, $c:tt) => {};
    ($n:ident, $c:tt $(, $ps:ident, $ts:ident, $is:tt)+) => {
        impl<I, $($ts: Shrink,)*> Shrink for Unify<orn::$n::Or<$($ts,)*>, I> where $($ts::Item: Into<I>,)* {
            type Item = I;

            fn item(&self) -> Self::Item {
                self.1.item().into()
            }

            fn shrink(&mut self) -> Option<Self> {
                Some(Unify(PhantomData, self.1.shrink()?))
            }
        }
    }
}

tuples!(tuple);
