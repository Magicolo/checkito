use crate::{
    generate::{Generator, State},
    shrink::Shrinker,
    utility::tuples,
};
use core::marker::PhantomData;

pub struct Fuse<T: ?Sized, I: ?Sized>(pub(crate) PhantomData<I>, pub(crate) T);

impl<T: Clone, I: ?Sized> Clone for Fuse<T, I> {
    fn clone(&self) -> Self {
        Self(PhantomData, self.1.clone())
    }
}

impl<G: Generator + ?Sized, I> Generator for Fuse<G, I>
where
    Fuse<G::Shrink, I>: Shrinker<Item = I>,
{
    type Item = I;
    type Shrink = Fuse<G::Shrink, I>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Fuse(PhantomData, self.1.generate(state))
    }

    fn constant(&self) -> bool {
        self.1.constant()
    }
}

macro_rules! tuple {
    ($n:ident, $c:tt) => {};
    ($n:ident, $c:tt $(, $ps:ident, $ts:ident, $is:tt)+) => {
        impl<I, $($ts: Shrinker,)*> Shrinker for Fuse<orn::$n::Or<$($ts,)*>, I> where $($ts::Item: Into<I>,)* {
            type Item = I;

            fn item(&self) -> Self::Item {
                self.1.item().into()
            }

            fn shrink(&mut self) -> Option<Self> {
                Some(Fuse(PhantomData, self.1.shrink()?))
            }
        }
    }
}

tuples!(tuple);
