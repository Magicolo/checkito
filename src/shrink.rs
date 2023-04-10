use crate::tuples;

pub trait FullShrink {
    type Item;
    type Shrink: Shrink<Item = Self::Item>;
    fn shrinker(item: Self::Item) -> Option<Self::Shrink>;
}

pub trait IntoShrink {
    type Item;
    type Shrink: Shrink<Item = Self::Item>;
    fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink>;
}

pub trait Shrink: Clone {
    type Item;

    fn item(&self) -> Self::Item;
    fn shrink(&mut self) -> Option<Self>;
}

impl<T: FullShrink> FullShrink for &T {
    type Item = T::Item;
    type Shrink = T::Shrink;

    fn shrinker(item: Self::Item) -> Option<Self::Shrink> {
        T::shrinker(item)
    }
}

impl<T: FullShrink> FullShrink for &mut T {
    type Item = T::Item;
    type Shrink = T::Shrink;

    fn shrinker(item: Self::Item) -> Option<Self::Shrink> {
        T::shrinker(item)
    }
}

impl<T: IntoShrink> IntoShrink for &T {
    type Item = T::Item;
    type Shrink = T::Shrink;

    fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
        T::shrinker(self, item)
    }
}

impl<T: IntoShrink> IntoShrink for &mut T {
    type Item = T::Item;
    type Shrink = T::Shrink;

    fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
        T::shrinker(self, item)
    }
}

macro_rules! tuple {
    ($n:ident, $c:tt $(,$p:ident, $t:ident, $i:tt)*) => {
        impl<$($t: FullShrink,)*> FullShrink for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type Shrink = ($($t::Shrink,)*);

            fn shrinker(_item: Self::Item) -> Option<Self::Shrink> {
                Some(($($t::shrinker(_item.$i)?,)*))
            }
        }

        impl<$($t: IntoShrink,)*> IntoShrink for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type Shrink = ($($t::Shrink,)*);

            fn shrinker(&self, _item: Self::Item) -> Option<Self::Shrink> {
                Some(($(self.$i.shrinker(_item.$i)?,)*))
            }
        }

        impl<$($t: Shrink,)*> Shrink for ($($t,)*) {
            type Item = ($($t::Item,)*);

            fn item(&self) -> Self::Item {
                ($(self.$i.item(),)*)
            }

            fn shrink(&mut self) -> Option<Self> {
                let mut _shrunk = false;
                let shrinks = ($(
                    if _shrunk { self.$i.clone() }
                    else {
                        match self.$i.shrink() {
                            Some(shrink) => { _shrunk = true; shrink },
                            None => self.$i.clone(),
                        }
                    },
                )*);
                if _shrunk { Some(shrinks) } else { None }
            }
        }
    };
}

tuples!(tuple);
