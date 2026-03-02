pub mod common;
use common::*;

macro_rules! tests {
    ($nonzero:ident, $inner:ident) => {
        mod $inner {
            use super::*;
            use core::num::$nonzero;

            #[test]
            fn generates_nonzero_values() {
                assert!($nonzero::generator()
                    .check(|n: $nonzero| n.get() != 0)
                    .is_none());
            }

            #[test]
            fn has_sample() {
                for i in 1..100 {
                    $nonzero::generator().samples(i).next().unwrap();
                }
            }

            #[test]
            fn sample_has_count() {
                for i in 0..100 {
                    assert_eq!($nonzero::generator().samples(i).len(), i);
                }
            }
        }
    };
}

tests!(NonZeroU8, u8);
tests!(NonZeroU16, u16);
tests!(NonZeroU32, u32);
tests!(NonZeroU64, u64);
tests!(NonZeroU128, u128);
tests!(NonZeroUsize, usize);
tests!(NonZeroI8, i8);
tests!(NonZeroI16, i16);
tests!(NonZeroI32, i32);
tests!(NonZeroI64, i64);
tests!(NonZeroI128, i128);
tests!(NonZeroIsize, isize);

#[test]
fn nonzero_u8_cardinality() {
    use core::num::NonZeroU8;
    assert_eq!(NonZeroU8::generator().cardinality(), Some(255));
}

#[test]
fn nonzero_i8_cardinality() {
    use core::num::NonZeroI8;
    // [-128, -1] = 128 values, [1, 127] = 127 values → 255 total
    assert_eq!(NonZeroI8::generator().cardinality(), Some(255));
}

#[check(_)]
fn nonzero_u8_check_attribute(n: core::num::NonZeroU8) {
    assert!(n.get() > 0);
}

#[check(_)]
fn nonzero_i32_check_attribute(n: core::num::NonZeroI32) {
    assert!(n.get() != 0);
}
