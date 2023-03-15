use crate::tuples;

pub trait Shrink: Clone {
    type Item;

    fn generate(&self) -> Self::Item;
    fn shrink(&mut self) -> Option<Self>;
}

macro_rules! tuple {
    ($n:ident, $c:tt $(,$p:ident, $t:ident, $i:tt)*) => {
        impl<$($t: Shrink,)*> Shrink for ($($t,)*) {
            type Item = ($($t::Item,)*);

            fn generate(&self) -> Self::Item {
                ($(self.$i.generate(),)*)
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
