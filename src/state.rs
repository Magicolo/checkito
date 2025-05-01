use crate::utility;
use core::{iter::FusedIterator, ops};
use fastrand::Rng;
use std::{
    mem::replace,
    ops::{Bound, RangeBounds},
};

#[derive(Clone, Copy, Debug)]
pub struct Sizes {
    range: Range<f64>,
    scale: f64,
}

#[derive(Clone, Debug)]
pub struct State {
    mode: Mode,
    sizes: Sizes,
    limit: usize,
    depth: usize,
    seed: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct States {
    indices: ops::Range<usize>,
    count: usize,
    sizes: Sizes,
    seed: u64,
}

#[derive(Copy, Clone, Debug)]
pub struct Range<T>(pub(crate) T, pub(crate) T);

pub struct With<'a> {
    state: &'a mut State,
    sizes: Sizes,
    depth: usize,
}

#[derive(Clone, Debug)]
enum Mode {
    // TODO: Can I use this for fuzzing? Add a `Fuzz(Box<dyn Iterator<Item = byte>>)`? Or
    // maybe fuzz through the `Random` object?
    Random(Rng),
    Exhaustive(u128),
}

impl State {
    pub(crate) fn random(index: usize, count: usize, size: Sizes, seed: u64) -> Self {
        Self {
            mode: Mode::Random(Rng::with_seed(seed.wrapping_add(index as _))),
            sizes: Sizes::from_ratio(index, count, size),
            limit: 0,
            depth: 0,
            seed,
        }
    }

    pub(crate) fn exhaustive(index: usize) -> Self {
        Self {
            mode: Mode::Exhaustive(index as _),
            sizes: Sizes::default(),
            limit: 0,
            depth: 0,
            seed: 0,
        }
    }

    #[inline]
    pub const fn size(&self) -> f64 {
        self.sizes.start()
    }

    #[inline]
    pub const fn scale(&self) -> f64 {
        self.sizes.scale
    }

    #[inline]
    pub const fn sizes(&self) -> Sizes {
        self.sizes
    }

    #[inline]
    pub const fn limit(&self) -> usize {
        self.limit
    }

    #[inline]
    pub const fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    #[inline]
    pub const fn with(&mut self) -> With {
        With::new(self)
    }

    #[inline]
    pub const fn descend(&mut self) -> With {
        let with = self.with();
        with.state.depth += 1;
        with.state.limit += 1;
        with
    }

    #[inline]
    pub const fn dampen(&mut self, deepest: usize, limit: usize, pressure: f64) -> With {
        let with = self.with();
        let old = with.state.sizes();
        let new = if with.state.depth >= deepest || with.state.limit >= limit {
            0.0
        } else {
            old.start() / utility::f64::max(with.state.depth as f64 * pressure, 1.0)
        };
        with.state.sizes = Sizes::new(new, old.end(), old.scale());
        with
    }

    #[inline]
    pub fn bool(&mut self) -> bool {
        self.u8(Range(0, 1)) == 1
    }

    #[inline]
    pub fn char<R: Into<Range<char>>>(&mut self, range: R) -> char {
        let Range(start, end) = range.into();
        let value = self.u32(Range(start as _, end as _));
        char::from_u32(value).unwrap_or(char::REPLACEMENT_CHARACTER)
    }
}

impl<'a> With<'a> {
    pub(crate) const fn new(state: &'a mut State) -> Self {
        Self {
            sizes: state.sizes(),
            depth: state.depth(),
            state,
        }
    }

    #[inline]
    pub const fn size(self, size: f64) -> Self {
        let scale = self.sizes.scale();
        self.sizes(Sizes::new(size, size, scale))
    }

    #[inline]
    pub const fn sizes(self, sizes: Sizes) -> Self {
        self.state.sizes = sizes;
        self
    }

    #[inline]
    pub const fn scale(self, scale: f64) -> Self {
        self.state.sizes.scale = scale;
        self
    }

    #[inline]
    pub const fn depth(self, depth: usize) -> Self {
        self.state.depth = depth;
        self
    }
}

impl ops::Deref for With<'_> {
    type Target = State;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.state
    }
}

impl ops::DerefMut for With<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state
    }
}

impl AsRef<State> for With<'_> {
    #[inline]
    fn as_ref(&self) -> &State {
        self.state
    }
}

impl AsMut<State> for With<'_> {
    #[inline]
    fn as_mut(&mut self) -> &mut State {
        self.state
    }
}

impl Drop for With<'_> {
    #[inline]
    fn drop(&mut self) {
        self.state.depth = self.depth;
        self.state.sizes = self.sizes;
    }
}

impl<T: Copy> Range<T> {
    #[inline]
    pub const fn start(&self) -> T {
        self.0
    }

    #[inline]
    pub const fn end(&self) -> T {
        self.1
    }
}

impl<T, R: Into<Range<T>> + Clone> From<&R> for Range<T> {
    fn from(value: &R) -> Self {
        value.clone().into()
    }
}

impl<T, R: Into<Range<T>> + Clone> From<&mut R> for Range<T> {
    fn from(value: &mut R) -> Self {
        value.clone().into()
    }
}

pub(crate) fn seed() -> u64 {
    fastrand::u64(..)
}

macro_rules! range {
    ($name: ident, $range: ty, $up: expr, $down: expr) => {
        impl From<$range> for Range<$name> {
            fn from(value: $range) -> Self {
                let mut start = match value.start_bound() {
                    Bound::Included(&bound) => (bound, false),
                    Bound::Excluded(&bound) => (bound, true),
                    Bound::Unbounded => ($name::MIN, false),
                };
                let mut end = match value.end_bound() {
                    Bound::Included(&bound) => (bound, false),
                    Bound::Excluded(&bound) => (bound, true),
                    Bound::Unbounded => ($name::MAX, false),
                };
                if start.0 > end.0 {
                    (start, end) = (end, start);
                }
                if start.1 {
                    start.0 = $up(start.0);
                }
                if end.1 {
                    end.0 = $down(end.0);
                }
                Self(
                    start.0.clamp($name::MIN, end.0),
                    end.0.clamp(start.0, $name::MAX),
                )
            }
        }
    };
}

macro_rules! ranges {
    ($name: ident, $up: expr, $down: expr) => {
        impl From<$name> for Range<$name> {
            fn from(value: $name) -> Self {
                Self(value, value)
            }
        }

        range!($name, ops::Range<$name>, $up, $down);
        range!($name, ops::RangeTo<$name>, $up, $down);
        range!($name, ops::RangeInclusive<$name>, $up, $down);
        range!($name, ops::RangeToInclusive<$name>, $up, $down);
        range!($name, ops::RangeFrom<$name>, $up, $down);
        range!($name, ops::RangeFull, $up, $down);
    };
}

macro_rules! integer {
    ($integer: ident, $positive: ident) => {
        ranges!($integer, |value| $integer::saturating_add(value, 1), |value| $integer::saturating_sub(value, 1));

        impl State {
            #[inline]
            pub fn $integer<R: Into<Range<$integer>>>(&mut self, range: R) -> $integer {
                #[inline]
                const fn divide(left: $positive, right: $positive) -> $positive {
                    let d = left / right;
                    let r = left % right;
                    if r > 0 {
                        d + 1
                    } else {
                        d
                    }
                }

                #[inline]
                fn shrink(range: $positive, size: f64, scale: f64) -> $positive {
                    if range == 0 || size <= 0.0 {
                        0
                    } else if size >= 1.0 {
                        range
                    } else {
                        // This adjustment of the size tries to prevent large ranges (such as `u64`)
                        // from rushing into huge values as soon as the `size > 0`.
                        let log = $positive::BITS - 1 - range.leading_zeros();
                        let power = size.powf(log as f64 / scale).recip();
                        divide(range, power as _)
                    }
                }

                fn generate(state: &mut State, Range(start, end): Range<$integer>) -> $integer {
                    let size = state.size();
                    let scale = state.scale();
                    match &mut state.mode {
                        Mode::Random(..) | Mode::Exhaustive(..) if start == end => start,
                        Mode::Random(random) => {
                            let range = shrink($positive::wrapping_sub(end as _, start as _), size, scale);
                            let value = random.$positive(0..=range) as $integer;
                            #[allow(unused_comparisons)]
                            if start >= 0 {
                                debug_assert!(end > 0);
                                start + value
                            } else if end <= 0 {
                                debug_assert!(start < 0);
                                end - value
                            } else {
                                debug_assert!(start < 0);
                                debug_assert!(end > 0);
                                // Centers the range around zero as much as possible.
                                let center = (range / 2) as $integer;
                                let shift = (start + center).max(0) + (end - center).min(0);
                                value.wrapping_add(shift).wrapping_sub(center)
                            }
                        }
                        // TODO: Generate 'small' values first. Maybe use the same adjustment as Random?
                        Mode::Exhaustive(index) => {
                            // The `saturating_add(1)` will cause the ranges `u128::MIN..=u128::MAX` and `i128::MIN..=i128::MAX` to never produce the values `u128::MAX` or `-1i128`.
                            // Considering that it would take `u128::MAX` iterations to reach that value, the inaccuracy is tolerated.
                            let range = u128::wrapping_sub(end as _, start as _).saturating_add(1);
                            let index = replace(index, *index / range) % range;
                            u128::wrapping_add(start as _, index) as $integer
                        }
                    }
                }
                generate(self, range.into())
            }
        }
    };
    ($([$integer: ident, $positive: ident]),*) => {
        $(integer!($integer, $positive);)*
    }
}

macro_rules! floating {
    ($number: ident, $bits: ident) => {
        ranges!($number, utility::$number::next_up, utility::$number::next_down);

        impl State {
            #[inline]
            pub fn $number<R: Into<Range<$number>>>(&mut self, range: R) -> $number {
                #[inline]
                fn shrink(range: $number, size: f64, scale: f64) -> $number {
                    if range == 0.0 || size <= 0.0 {
                        0.0
                    } else if size >= 1.0 {
                        range
                    } else {
                        let log = range.abs().log2();
                        let power = size.powf(log as f64 / scale);
                        range * power as $number
                    }
                }

                fn generate(state: &mut State, Range(start, end): Range<$number>) -> $number {
                    assert!(start.is_finite() && end.is_finite());

                    let size = state.size();
                    let scale = state.scale();
                    match &mut state.mode {
                        Mode::Random(..) | Mode::Exhaustive(..) if start == end => start,
                        Mode::Random(random) => {
                            if start >= 0.0 {
                                debug_assert!(end > 0.0);
                                start + random.$number() * shrink(end - start, size, scale)
                            } else if end <= 0.0 {
                                debug_assert!(start < 0.0);
                                end - random.$number() * shrink(end - start, size, scale)
                            } else {
                                debug_assert!(start < 0.0);
                                debug_assert!(end > 0.0);
                                // Chooses either the positive or negative range based on the ratio between the 2.
                                let (small, big) = if -start < end { (start, end) } else { (end, start) };
                                let ratio = (small / big).abs().clamp(1e-3, 1e3);
                                let random = random.$number() * (1.0 + ratio);
                                if random <= 1.0 {
                                    random * shrink(big, size, scale)
                                } else {
                                    (random - 1.0) / ratio * shrink(small, size, scale)
                                }
                            }
                        }
                        // TODO: Generate 'small' values first. Maybe use the same adjustment as Random?
                        Mode::Exhaustive(index) => {
                            let start = utility::$number::to_bits(start);
                            let end = utility::$number::to_bits(end);
                            let range = u128::wrapping_sub(end as _, start as _).saturating_add(1);
                            let index = replace(index, *index / range) % range;
                            let bits = u128::wrapping_add(start as _, index);
                            utility::$number::from_bits(bits as _)
                        }
                    }
                }
                generate(self, range.into())
            }
        }
    };
    ($([$number: ident, $bits: ident]),*) => {
        $(floating!($number, $bits);)*
    }
}
ranges!(
    char,
    |value: char| char::from_u32(u32::saturating_add(value as _, 1))
        .unwrap_or(char::REPLACEMENT_CHARACTER),
    |value: char| char::from_u32(u32::saturating_sub(value as _, 1))
        .unwrap_or(char::REPLACEMENT_CHARACTER)
);
integer!(
    [u8, u8],
    [u16, u16],
    [u32, u32],
    [u64, u64],
    [u128, u128],
    [usize, usize],
    [i8, u8],
    [i16, u16],
    [i32, u32],
    [i64, u64],
    [i128, u128],
    [isize, usize]
);

floating!([f32, i32], [f64, i64]);

impl States {
    pub fn new<S: Into<Sizes>>(count: usize, size: S, seed: Option<u64>) -> Self {
        Self {
            indices: 0..count,
            count,
            sizes: size.into(),
            seed: seed.unwrap_or_else(self::seed),
        }
    }
}

impl Iterator for States {
    type Item = State;

    fn next(&mut self) -> Option<Self::Item> {
        Some(State::random(
            self.indices.next()?,
            self.count,
            self.sizes,
            self.seed,
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }

    fn count(self) -> usize {
        self.indices.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        Some(State::random(
            self.indices.nth(n)?,
            self.count,
            self.sizes,
            self.seed,
        ))
    }

    fn last(mut self) -> Option<Self::Item> {
        Some(State::random(
            self.indices.next()?,
            self.count,
            self.sizes,
            self.seed,
        ))
    }
}

impl ExactSizeIterator for States {
    fn len(&self) -> usize {
        self.indices.len()
    }
}

impl DoubleEndedIterator for States {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some(State::random(
            self.indices.next_back()?,
            self.count,
            self.sizes,
            self.seed,
        ))
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        Some(State::random(
            self.indices.nth_back(n)?,
            self.count,
            self.sizes,
            self.seed,
        ))
    }
}

impl FusedIterator for States {}

impl Sizes {
    const SCALE: f64 = 6.0;

    #[inline]
    pub(crate) const fn new(start: f64, end: f64, scale: f64) -> Self {
        assert!(start.is_finite() && end.is_finite() && start <= end);
        assert!(scale.is_finite() && scale >= 1.0);

        Self {
            range: Range(
                utility::f64::clamp(start, 0.0, 1.0),
                utility::f64::clamp(end, 0.0, 1.0),
            ),
            scale: utility::f64::clamp(scale, 1.0, f64::MAX),
        }
    }

    #[inline]
    pub(crate) const fn from_ratio(index: usize, count: usize, size: Self) -> Self {
        let (start, end) = (size.start(), size.end());
        if count <= 1 {
            Self::new(end, end, Self::SCALE)
        } else {
            let range = end - start;
            // This size calculation ensures that 25% of samples are fully sized.
            let ratio = index as f64 / count as f64 * 1.25;
            let size = utility::f64::clamp(start + ratio * range, 0.0, end);
            Self::new(size, end, Self::SCALE)
        }
    }

    #[inline]
    pub const fn scale(&self) -> f64 {
        self.scale
    }

    #[inline]
    pub const fn start(&self) -> f64 {
        self.range.0
    }

    #[inline]
    pub const fn end(&self) -> f64 {
        self.range.1
    }
}

impl Default for Sizes {
    fn default() -> Self {
        Self::new(0.0, 1.0, Self::SCALE)
    }
}

impl<R: Into<Range<f64>>> From<R> for Sizes {
    fn from(value: R) -> Self {
        let range = value.into();
        Self::new(range.start(), range.end(), Self::SCALE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cmp::Ordering;

    #[test]
    fn random_is_exhaustive() {
        fn check<T: Ord>(count: usize, generate: impl Fn(&mut State) -> T) {
            check_with(count, generate, T::cmp);
        }

        fn check_with<T>(
            count: usize,
            generate: impl Fn(&mut State) -> T,
            compare: impl Fn(&T, &T) -> Ordering,
        ) {
            let mut state = State::random(1, 1, 1.0.into(), 0);
            let mut values = (0..count * 25)
                .map(|_| generate(&mut state))
                .collect::<Vec<_>>();
            values.sort_by(&compare);
            values.dedup_by(|left, right| compare(left, right) == Ordering::Equal);
            assert_eq!(values.len(), count);
        }

        check(2, |state| state.bool());
        check(26, |state| state.char('a'..='z'));
        check(8, |state| state.char('1'..'9'));
        check(256, |state| state.u8(..));
        check(256, |state| state.i8(..));
        check(65536, |state| state.u16(..));
        check(32768, |state| state.i16(..0));
        check(1000, |state| state.isize(isize::MIN..isize::MIN + 1000));
        check(1001, |state| state.u128(u128::MAX - 1000..=u128::MAX));
        check_with(16385, |state| state.f32(-1000.0..=-999.0), compare_f32);
        check_with(1430, |state| state.f32(-1e-42..=1e-42), compare_f32);
    }

    #[test]
    fn exhaustive_is_exhaustive() {
        fn check<T: Ord>(count: usize, generate: impl Fn(&mut State) -> T) {
            check_with(count, generate, T::cmp);
        }

        fn check_with<T>(
            count: usize,
            generate: impl Fn(&mut State) -> T,
            compare: impl Fn(&T, &T) -> Ordering,
        ) {
            let mut values = (0..count)
                .map(|i| generate(&mut State::exhaustive(i as _)))
                .collect::<Vec<_>>();
            values.sort_by(&compare);
            values.dedup_by(|left, right| compare(left, right) == Ordering::Equal);
            assert_eq!(values.len(), count);
        }

        check(2, |state| state.bool());
        check(26, |state| state.char('a'..='z'));
        check(8, |state| state.char('1'..'9'));
        check(256, |state| state.u8(..));
        check(256, |state| state.i8(..));
        check(65536, |state| state.u16(..));
        check(32768, |state| state.i16(..0));
        check(1000, |state| state.isize(isize::MIN..isize::MIN + 1000));
        check(1001, |state| state.u128(u128::MAX - 1000..=u128::MAX));
        check_with(16385, |state| state.f32(-1000.0..=-999.0), compare_f32);
        check_with(1430, |state| state.f32(-1e-42..=1e-42), compare_f32);
    }

    fn compare_f32(left: &f32, right: &f32) -> Ordering {
        let left = utility::f32::to_bits(*left);
        let right = utility::f32::to_bits(*right);
        u32::cmp(&left, &right)
    }
}
