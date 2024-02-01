use crate::{boxed, utility::tuples};
use std::num::NonZeroUsize;

#[derive(Clone, Debug)]
pub struct All<T: ?Sized> {
    pub index: usize,
    pub inner: T,
}

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

    fn boxed(self) -> boxed::Shrinker<Self::Item>
    where
        Self: 'static,
    {
        boxed::Shrinker::new(self)
    }
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

impl<T> All<T> {
    pub const fn new(inner: T) -> Self {
        Self { inner, index: 0 }
    }
}

macro_rules! tuple {
    ($n:ident, $c:tt $(,$p:ident, $t:ident, $i:tt)*) => {
        impl<$($t: FullShrink,)*> FullShrink for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type Shrink = All<($($t::Shrink,)*)>;

            fn shrinker(_item: Self::Item) -> Option<Self::Shrink> {
                Some(All::new(($($t::shrinker(_item.$i)?,)*)))
            }
        }

        impl<$($t: IntoShrink,)*> IntoShrink for ($($t,)*) {
            type Item = ($($t::Item,)*);
            type Shrink = All<($($t::Shrink,)*)>;

            fn shrinker(&self, _item: Self::Item) -> Option<Self::Shrink> {
                Some(All::new(($(self.$i.shrinker(_item.$i)?,)*)))
            }
        }

        impl<$($t: Shrink,)*> Shrink for All<($($t,)*)> {
            type Item = ($($t::Item,)*);

            #[allow(clippy::unused_unit)]
            fn item(&self) -> Self::Item {
                ($(self.inner.$i.item(),)*)
            }

            fn shrink(&mut self) -> Option<Self> {
                let count = NonZeroUsize::new($c)?;
                let start = self.index;
                self.index += 1;
                for i in 0..count.get() {
                    let index = (start + i) % count.get();
                    match index {
                        $($i => {
                            if let Some(shrink) = self.inner.$i.shrink() {
                                let mut shrinks = self.inner.clone();
                                shrinks.$i = shrink;
                                return Some(Self {
                                    inner: shrinks,
                                    index: self.index
                                });
                            }
                            })*
                        _ => {}
                    }
                }
                None
            }
        }
    };
}

tuples!(tuple);
