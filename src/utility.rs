pub(crate) mod cardinality {
    #[inline]
    pub(crate) const fn all_product(left: Option<usize>, right: Option<usize>) -> Option<usize> {
        match (left, right) {
            (Some(0), _) | (_, Some(0)) => Some(0),
            (Some(left), Some(right)) => usize::checked_mul(left, right),
            (None, _) | (_, None) => None,
        }
    }

    #[inline]
    pub(crate) const fn all_repeat_static<const N: usize>(value: Option<usize>) -> Option<usize> {
        match (value, N) {
            (_, 0) => Some(1),
            (Some(0), _) => Some(0),
            (Some(value), count) => {
                if count <= u32::MAX as _ {
                    usize::checked_pow(value, count as _)
                } else {
                    None
                }
            }
            (None, _) => None,
        }
    }

    // pub(crate) const fn all_repeat_dynamic(
    //     value: Option<usize>,
    //     count: Option<usize>,
    // ) -> Option<usize> {
    //     // FIXME: This considers only all values of [T; count] but not [T; count
    // - 1]     // (and so on). Example: when T = true, count = 2, the possible
    // values are [],     // [true], [true, true]. This is not represented here.
    //     match (value, count) {
    //         (_, Some(0)) => Some(1),
    //         (Some(0), _) => Some(0),
    //         (Some(1), Some(count @ 1..)) => usize::checked_add(count, 1),
    //         (Some(value @ 2..), Some(count @ 1..)) => {
    //             if count <= u32::MAX as _ {
    //                 if let Some(result) = usize::checked_pow(value, count as _) {
    //                     usize::checked_mul(result, value / (value - 1))
    //                 } else {
    //                     None
    //                 }
    //             } else {
    //                 None
    //             }
    //         }
    //         (None, _) | (_, None) => None,
    //     }
    // }

    pub(crate) const fn any_sum(left: Option<usize>, right: Option<usize>) -> Option<usize> {
        match (left, right) {
            (Some(left), Some(right)) => usize::checked_add(left, right),
            (None, _) | (_, None) => None,
        }
    }
}

pub(crate) mod f32 {
    const SIGN_MASK: u32 = 0x8000_0000;
    const TINY_BITS: u32 = 0x1;
    const NEG_TINY_BITS: u32 = TINY_BITS | SIGN_MASK;

    pub const fn to_bits(value: f32) -> u32 {
        let bits = f32::to_bits(value);
        if bits & SIGN_MASK != 0 {
            !bits
        } else {
            bits | SIGN_MASK
        }
    }

    #[inline]
    pub const fn from_bits(bits: u32) -> f32 {
        let value = if bits & SIGN_MASK != 0 {
            bits & !SIGN_MASK
        } else {
            !bits
        };
        f32::from_bits(value)
    }

    pub const fn cardinality(start: f32, end: f32) -> u128 {
        if start.is_nan() || end.is_nan() {
            1
        } else {
            u128::wrapping_sub(to_bits(end) as _, to_bits(start) as _).saturating_add(1)
        }
    }

    /// Copied from 'https://doc.rust-lang.org/src/core/num/f32.rs.html' to continue supporting lower rust versions.
    #[inline]
    pub const fn next_up(value: f32) -> f32 {
        let bits = value.to_bits();
        if value.is_nan() || bits == f32::INFINITY.to_bits() {
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

        f32::from_bits(next_bits)
    }

    /// Copied from 'https://doc.rust-lang.org/src/core/num/f32.rs.html' to continue supporting lower rust versions.
    #[inline]
    pub const fn next_down(value: f32) -> f32 {
        let bits = value.to_bits();
        if value.is_nan() || bits == f32::NEG_INFINITY.to_bits() {
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

        f32::from_bits(next_bits)
    }
}

pub(crate) mod f64 {
    const SIGN_MASK: u64 = 0x8000_0000_0000_0000;
    const TINY_BITS: u64 = 0x1;
    const NEG_TINY_BITS: u64 = TINY_BITS | SIGN_MASK;

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

    #[inline]
    pub const fn max(left: f64, right: f64) -> f64 {
        if left >= right { left } else { right }
    }

    #[inline]
    pub const fn to_bits(value: f64) -> u64 {
        let bits = f64::to_bits(value);
        if bits & SIGN_MASK != 0 {
            !bits
        } else {
            bits | SIGN_MASK
        }
    }

    #[inline]
    pub const fn from_bits(bits: u64) -> f64 {
        let value = if bits & SIGN_MASK != 0 {
            bits & !SIGN_MASK
        } else {
            !bits
        };
        f64::from_bits(value)
    }

    #[inline]
    pub const fn cardinality(start: f64, end: f64) -> u128 {
        if start.is_nan() || end.is_nan() {
            1
        } else {
            u128::wrapping_sub(to_bits(end) as _, to_bits(start) as _).saturating_add(1)
        }
    }

    /// Copied from 'https://doc.rust-lang.org/src/core/num/f64.rs.html' to continue supporting lower rust versions.
    #[inline]
    pub const fn next_up(value: f64) -> f64 {
        let bits = value.to_bits();
        if value.is_nan() || bits == f64::INFINITY.to_bits() {
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

        f64::from_bits(next_bits)
    }

    /// Copied from 'https://doc.rust-lang.org/src/core/num/f64.rs.html' to continue supporting lower rust versions.
    #[inline]
    pub const fn next_down(value: f64) -> f64 {
        let bits = value.to_bits();
        if value.is_nan() || bits == f64::NEG_INFINITY.to_bits() {
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

        f64::from_bits(next_bits)
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
