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

/// Macro to implement floating-point bit manipulation utilities for f32 and f64.
///
/// This eliminates code duplication between the f32 and f64 modules by generating
/// the same set of functions with different type parameters and bit sizes.
macro_rules! impl_float_bits {
    ($float_type:ty, $bits_type:ty, $sign_mask:expr) => {
        const SIGN_MASK: $bits_type = $sign_mask;
        const TINY_BITS: $bits_type = 0x1;
        const NEG_TINY_BITS: $bits_type = TINY_BITS | SIGN_MASK;

        /// Converts a float to bits in a total-order representation.
        ///
        /// This transformation ensures that bit-level comparison matches
        /// numerical comparison, handling negative numbers and NaN correctly.
        #[inline]
        pub const fn to_bits(value: $float_type) -> $bits_type {
            let bits = <$float_type>::to_bits(value);
            if bits & SIGN_MASK != 0 {
                !bits
            } else {
                bits | SIGN_MASK
            }
        }

        /// Converts bits in total-order representation back to a float.
        #[inline]
        pub const fn from_bits(bits: $bits_type) -> $float_type {
            let bits = if bits & SIGN_MASK != 0 {
                bits & !SIGN_MASK
            } else {
                !bits
            };
            <$float_type>::from_bits(bits)
        }

        /// Calculates the cardinality (number of distinct values) in a float range.
        ///
        /// Returns `Some(1)` for NaN values, otherwise computes the difference
        /// in bit representations plus one.
        #[inline]
        pub const fn cardinality(start: $float_type, end: $float_type) -> Option<u128> {
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
        pub const fn next_up(value: $float_type) -> $float_type {
            let bits = value.to_bits();
            if value.is_nan() || bits == <$float_type>::INFINITY.to_bits() {
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

            <$float_type>::from_bits(next_bits)
        }

        /// Returns the next representable value below the given float.
        ///
        /// Copied from Rust's stdlib to support older Rust versions.
        #[inline]
        pub const fn next_down(value: $float_type) -> $float_type {
            let bits = value.to_bits();
            if value.is_nan() || bits == <$float_type>::NEG_INFINITY.to_bits() {
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

            <$float_type>::from_bits(next_bits)
        }
    };
}

pub(crate) mod f32 {
    impl_float_bits!(f32, u32, 0x8000_0000);
}

pub(crate) mod f64 {
    impl_float_bits!(f64, u64, 0x8000_0000_0000_0000);

    /// Clamps a value between a minimum and maximum.
    #[inline]
    pub const fn clamp(value: f64, low: f64, high: f64) -> f64 {
        if value < low {
            low
        } else if value > high {
            high
        } else {
            value
        }
    }

    /// Returns the maximum of two f64 values.
    #[inline]
    pub const fn max(left: f64, right: f64) -> f64 {
        if left >= right { left } else { right }
    }
}

macro_rules! tuples {
    ($m:ident) => {
        $m!(or0, 0);
        $m!(or1, 1, p0, T0, 0);
        $m!(or2, 2, p0, T0, 0, p1, T1, 1);
        $m!(or3, 3, p0, T0, 0, p1, T1, 1, p2, T2, 2);
        $m!(or4, 4, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3);
        $m!(
            or5, 5, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4
        );
        $m!(
            or6, 6, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5
        );
        $m!(
            or7, 7, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6
        );
        $m!(
            or8, 8, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6,
            p7, T7, 7
        );
        $m!(
            or9, 9, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6,
            p7, T7, 7, p8, T8, 8
        );
        $m!(
            or10, 10, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6,
            p7, T7, 7, p8, T8, 8, p9, T9, 9
        );
        $m!(
            or11, 11, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6,
            p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10
        );
        $m!(
            or12, 12, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6,
            p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11
        );
        $m!(
            or13, 13, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6,
            p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12
        );
        $m!(
            or14, 14, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6,
            p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13
        );
        $m!(
            or15, 15, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6,
            p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14
        );
        $m!(
            or16, 16, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6,
            p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15
        );
    };
}

pub(crate) use tuples;
