pub mod common;
use common::*;
use orn::{Or1, Or2, Or3, Or4};

pub fn is_generator<T>(_: impl Generate<Item = T>) {}

macro_rules! generators {
    ($type: ident, $value: expr, $or: ty, $($values: expr),+) => {
        mod $type {
            use super::*;

            #[test]
            fn prelude_generators_implement_generate() {
                is_generator::<$type>(same($value));
                is_generator::<Option<$type>>(any([$($values),*]));
                is_generator::<$or>(any(($($values,)*)));
                is_generator::<$type>(unify(any(($($values,)*))));
                is_generator::<$type>(map($value, |value| value));
                is_generator::<$type>(flat_map($value, same));
                is_generator::<$type>(flatten(same(same($value))));
                is_generator::<Option<$type>>(filter($value, |_| true, 1));
                is_generator::<Option<$type>>(filter_map($value, Some, 1));
                is_generator::<$type>(boxed(Box::new($value)));
                is_generator::<[$type; 1]>(array::<_, 1>($value));
                is_generator::<Vec<$type>>(collect($value, 1usize));
                is_generator::<$type>(size($value, |_| 1.0));
                is_generator::<$type>(dampen($value, 1.0, 1, 1));
                is_generator::<$type>(keep($value));
                is_generator::<Option<$type>>(convert($value));
                is_generator::<same::Same<$type>>(shrinker(same($value)));
                is_generator::<keep::Keep<$type>>(shrinker(keep($value)));
                is_generator::<convert::Convert<$type, $type>>(shrinker(convert($value)));
            }
        }
    };
}

generators!(u8, 1u8, Or1<u8>, 2u8);
generators!(i32, 1i32, Or2<i32, i32>, 2i32, 3i32);
generators!(char, 'a', Or3<char, char, char>, 'b', 'c', 'd');
generators!(bool, true, Or4<bool, bool, bool, bool>, false, true, false, false);
