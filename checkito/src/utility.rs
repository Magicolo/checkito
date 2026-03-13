use core::any::Any;
use std::borrow::Cow;

pub(crate) fn cast(
    error: Box<dyn Any + Send + 'static>,
) -> Result<Cow<'static, str>, Box<dyn Any + Send + 'static>> {
    let error = match error.downcast::<&'static str>() {
        Ok(error) => return Ok(Cow::Borrowed(*error)),
        Err(error) => error,
    };
    let error = match error.downcast::<String>() {
        Ok(error) => return Ok(Cow::Owned(*error)),
        Err(error) => error,
    };
    let error = match error.downcast::<Box<str>>() {
        Ok(error) => return Ok(Cow::Owned(error.to_string())),
        Err(error) => error,
    };
    let error = match error.downcast::<Cow<'static, str>>() {
        Ok(error) => return Ok(*error),
        Err(error) => error,
    };
    Err(error)
}

/// Macro to implement floating-point bit manipulation utilities for f32 and
/// f64.
///
/// This eliminates code duplication between the f32 and f64 modules by
/// generating the same set of functions with different type parameters and bit
/// sizes.
macro_rules! float {
    ($type:ty, $bits:ty, $mask:expr) => {
        const SIGN_MASK: $bits = $mask;
        const TINY_BITS: $bits = 0x1;
        const NEG_TINY_BITS: $bits = TINY_BITS | SIGN_MASK;

        /// Converts a float to bits in a total-order representation.
        ///
        /// This transformation ensures that bit-level comparison matches
        /// numerical comparison, handling negative numbers and NaN correctly.
        #[inline]
        pub const fn to_bits(value: $type) -> $bits {
            let bits = <$type>::to_bits(value);
            if bits & SIGN_MASK != 0 {
                !bits
            } else {
                bits | SIGN_MASK
            }
        }

        /// Converts bits in total-order representation back to a float.
        #[inline]
        pub const fn from_bits(bits: $bits) -> $type {
            let bits = if bits & SIGN_MASK != 0 {
                bits & !SIGN_MASK
            } else {
                !bits
            };
            <$type>::from_bits(bits)
        }

        /// Calculates the cardinality (number of distinct values) in a float range.
        ///
        /// Returns `Some(1)` for NaN values, otherwise computes the difference
        /// in bit representations plus one.
        #[inline]
        pub const fn cardinality(start: $type, end: $type) -> Option<u128> {
            if start.is_nan() || end.is_nan() {
                Some(1)
            } else {
                u128::wrapping_sub(to_bits(end) as _, to_bits(start) as _).checked_add(1)
            }
        }

        /// Returns the next representable value above the given float.
        ///
        /// Copied from Rust's stdlib to support older Rust versions.
        #[inline]
        pub const fn next_up(value: $type) -> $type {
            let bits = value.to_bits();
            if value.is_nan() || bits == <$type>::INFINITY.to_bits() {
                return value;
            }

            let abs = bits & !SIGN_MASK;
            let next_bits = if abs == 0 {
                TINY_BITS
            } else if bits == abs {
                bits + 1
            } else {
                bits - 1
            };

            <$type>::from_bits(next_bits)
        }

        /// Returns the next representable value below the given float.
        ///
        /// Copied from Rust's stdlib to support older Rust versions.
        #[inline]
        pub const fn next_down(value: $type) -> $type {
            let bits = value.to_bits();
            if value.is_nan() || bits == <$type>::NEG_INFINITY.to_bits() {
                return value;
            }

            let abs = bits & !SIGN_MASK;
            let next_bits = if abs == 0 {
                NEG_TINY_BITS
            } else if bits == abs {
                bits - 1
            } else {
                bits + 1
            };

            <$type>::from_bits(next_bits)
        }

        #[inline]
        #[allow(dead_code)]
        pub const fn clamp(value: $type, low: $type, high: $type) -> $type {
            if value < low {
                low
            } else if value > high {
                high
            } else {
                value
            }
        }

        #[inline]
        #[allow(dead_code)]
        pub const fn max(left: $type, right: $type) -> $type {
            if left >= right { left } else { right }
        }

        #[inline]
        #[allow(dead_code)]
        pub const fn min(left: $type, right: $type) -> $type {
            if left <= right { left } else { right }
        }
    };
}

pub(crate) mod f32 {
    float!(f32, u32, 0x8000_0000);
}

pub(crate) mod f64 {
    float!(f64, u64, 0x8000_0000_0000_0000);
}

macro_rules! tuples {
    ($m: ident) => {
        tuples!(@recurse $m
            [or0, 0, or1, 1, or2, 2, or3, 3, or4, 4, or5, 5, or6, 6, or7, 7, or8, 8, or9, 9, or10, 10, or11, 11, or12, 12, or13, 13, or14, 14, or15, 15,]
            []
            [p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13, 13, p14, T14, 14, p15, T15, 15,]);
    };
    (@recurse $m: ident [] [$($ps: ident, $ts: ident, $is: tt,)*] []) => {};
    (@recurse $m: ident
        [$([$e: tt],)? $n: ident, $c: tt, $($([$es: tt],)? $ns: ident, $cs: tt,)*]
        [$($ps: ident, $ts: ident, $is: tt,)*]
        [$q: ident, $u: ident, $j: tt, $($qs: ident, $us: ident, $js: tt,)*]) => {
        $m!($n, $c $(, $e)? $(, $ps, $ts, $is)*);
        tuples!(@recurse $m [$($([$es],)? $ns, $cs,)*] [$($ps, $ts, $is,)* $q, $u, $j,] [$($qs, $us, $js,)*]);
    };
}

pub(crate) use tuples;
