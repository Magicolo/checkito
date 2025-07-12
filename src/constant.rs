use crate::{
    Generate,
    collect::Count,
    primitive::{Shrinker, bool, char},
    state::{self, State},
};
use core::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
pub struct Range<T: Constant + ?Sized, U: Constant + ?Sized>(PhantomData<T>, PhantomData<U>);

pub trait Constant {
    type Item;
    const VALUE: Self::Item;
}

impl<T: Constant + ?Sized, U: Constant + ?Sized> Range<T, U> {
    pub const fn new() -> Self {
        Self(PhantomData, PhantomData)
    }
}

impl<T: Constant + ?Sized, U: Constant + ?Sized> Default for Range<T, U> {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! wrap {
    ($type: ident, $name: ident) => {
        #[derive(Debug, Copy, Clone)]
        pub struct $name<const N: $type>;

        impl<const N: $type> Constant for $name<N> {
            type Item = $type;

            const VALUE: Self::Item = N;
        }

        impl<const N: $type> Generate for $name<N> {
            type Item = $type;
            type Shrink = $type;

            const CARDINALITY: Option<u128> = Some(1);

            fn generate(&self, _: &mut State) -> Self::Shrink {
                N
            }
        }
    };
}

macro_rules! range {
    ($type: ident, $name: ident, $shrink: ty) => {
        wrap!($type, $name);

        impl<const N: $type, const M: $type> Generate for Range<$name<N>, $name<M>> {
            type Item = $type;
            type Shrink = $shrink;

            const CARDINALITY: Option<u128> = u128::checked_sub(M as _, N as _);

            fn generate(&self, state: &mut State) -> Self::Shrink {
                state::Range(N, M).generate(state)
            }
        }
    };
    ($([$type: ident, $name: ident]),*$(,)?) => {
        $(range!($type, $name, Shrinker<$type>);)*
    };
}

range!(
    [u8, U8],
    [u16, U16],
    [u32, U32],
    [u64, U64],
    [u128, U128],
    [usize, Usize],
    [i8, I8],
    [i16, I16],
    [i32, I32],
    [i64, I64],
    [i128, I128],
    [isize, Isize],
);

range!(char, Char, char::Shrinker);
wrap!(bool, Bool);

impl<const N: usize> Count for Usize<N> {
    const COUNT: Option<state::Range<usize>> = Some(state::Range(N, N));

    fn count(&self) -> state::Range<usize> {
        state::Range(N, N)
    }
}

impl<const N: usize, const M: usize> Count for Range<Usize<N>, Usize<M>> {
    const COUNT: Option<state::Range<usize>> = Some(state::Range(N, M));

    fn count(&self) -> state::Range<usize> {
        state::Range(N, M)
    }
}
