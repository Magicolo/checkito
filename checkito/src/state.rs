use crate::{
    GENERATES, Generate, Shrink,
    primitive::{Range, u8::U8},
    utility,
};
use core::{
    iter::{FusedIterator, from_fn},
    mem::{replace, take},
    ops::{self, Bound},
};
use fastrand::Rng;
use orn::Or3;
use std::ops::RangeBounds;

#[derive(Clone, Copy, Debug)]
pub struct Sizes {
    range: Range<f64>,
    scale: f64,
}

#[derive(Clone, Debug)]
pub struct State {
    mode: Mode,
    sizes: Sizes,
    index: usize,
    count: usize,
    limit: usize,
    depth: usize,
    seed: u64,
}

#[derive(Clone, Debug)]
pub struct States {
    indices: ops::Range<usize>,
    modes: Modes,
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum Modes {
    Random {
        count: usize,
        sizes: Sizes,
        seed: u64,
    },
    Exhaustive(usize),
}

#[derive(Clone, Debug)]
pub struct Weight<T: ?Sized> {
    weight: f64,
    generator: T,
}

impl<T> Weight<T> {
    pub const fn weight(&self) -> f64 {
        self.weight
    }

    pub const fn value(&self) -> &T {
        &self.generator
    }
}

impl<G: Generate> Weight<G> {
    pub fn new(weight: f64, generator: G) -> Self {
        assert!(weight.is_finite());
        assert!(weight >= f64::EPSILON);
        Self { weight, generator }
    }
}

impl<G: Generate + ?Sized> Weight<G> {
    pub(crate) fn cardinality(&self) -> Option<u128> {
        self.generator.cardinality()
    }
}

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
            index,
            count,
            limit: 0,
            depth: 0,
            seed,
        }
    }

    pub(crate) const fn exhaustive(index: usize, count: usize) -> Self {
        Self {
            mode: Mode::Exhaustive(index as _),
            sizes: Sizes::DEFAULT,
            index,
            count,
            limit: 0,
            depth: 0,
            seed: 0,
        }
    }

    pub(crate) fn any_exhaustive<I: IntoIterator<Item: Generate, IntoIter: Clone>>(
        index: &mut u128,
        generators: I,
    ) -> Option<I::Item> {
        for generator in generators.into_iter().cycle() {
            match generator.cardinality() {
                Some(cardinality) if *index <= cardinality => {
                    return Some(generator);
                }
                Some(cardinality) => *index -= cardinality,
                None => return Some(generator),
            }
        }
        None
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
    pub const fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub const fn count(&self) -> usize {
        self.count
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
        self.u8(Range(U8::ZERO, U8::ONE)) == 1
    }

    #[inline]
    pub fn char<R: Into<Range<char>>>(&mut self, range: R) -> char {
        let Range(start, end) = range.into();
        let value = self.u32(Range(start as u32, end as u32));
        char::from_u32(value).unwrap_or(char::REPLACEMENT_CHARACTER)
    }

    pub(crate) fn any_indexed<'a, G: Generate>(&mut self, generators: &'a [G]) -> Option<&'a G> {
        let end = generators.len().checked_sub(1)?;
        match &mut self.mode {
            Mode::Random(_) => {
                let index = self.with().size(1.0).usize(Range(0, end));
                generators.get(index)
            }
            Mode::Exhaustive(index) => Self::any_exhaustive(index, generators),
        }
    }

    pub(crate) fn any_weighted<'a, G: Generate>(
        &mut self,
        generators: &'a [Weight<G>],
    ) -> Option<&'a G> {
        if generators.is_empty() {
            return None;
        }

        match &mut self.mode {
            Mode::Random(_) => {
                let total = generators
                    .iter()
                    .map(|Weight { weight, .. }| weight)
                    .sum::<f64>()
                    .min(f64::MAX);
                debug_assert!(total > 0.0 && total.is_finite());
                let mut random = self.with().size(1.0).f64(0.0..=total);
                debug_assert!(random.is_finite());
                for Weight {
                    weight,
                    generator: value,
                } in generators
                {
                    if random < *weight {
                        return Some(value);
                    } else {
                        random -= weight;
                    }
                }
                unreachable!(
                    "there is at least one item in the slice and weights are finite and `> 0.0`"
                );
            }
            Mode::Exhaustive(index) => {
                Self::any_exhaustive(index, generators.iter().map(Weight::value))
            }
        }
    }

    // TODO: Implement `any_tuple_indexed` and `any_tuple_weighted`...

    pub(crate) fn repeat<'a, 'b, G: Generate + ?Sized>(
        &'a mut self,
        generator: &'b G,
        range: Range<usize>,
    ) -> impl Iterator<Item = G::Shrink> + use<'a, 'b, G> {
        match &mut self.mode {
            Mode::Random(_) => {
                let count = range.generate(self).item();
                Or3::T0(Iterator::map(0..count, move |_| generator.generate(self)))
            }
            Mode::Exhaustive(0) => Or3::T1([] as [G::Shrink; 0]),
            Mode::Exhaustive(index @ 1..) => {
                *index -= 1;
                let mut first = true;
                Or3::T2(from_fn(move || match &mut self.mode {
                    _ if take(&mut first) => Some(generator.generate(self)),
                    Mode::Exhaustive(1..) => Some(generator.generate(self)),
                    _ => None,
                }))
            }
        }
        .into_iter()
        .map(|or| or.into())
    }
}

const fn consume(index: &mut u128, start: u128, end: u128) -> u128 {
    let range = u128::wrapping_sub(end as _, start as _).saturating_add(1);
    let index = replace(index, index.saturating_div(range)) % range;
    u128::wrapping_add(start as _, index)
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
    ($integer: ident, $positive: ident, $constant: ident) => {
        ranges!($integer, |value| $integer::saturating_add(value, 1), |value| $integer::saturating_sub(value, 1));

        impl State {
            #[inline]
            pub fn $integer<R: Into<Range<$integer>>>(&mut self, range: R) -> $integer {
                #[inline]
                const fn divide(left: $positive, right: $positive) -> $positive {
                    let value = left / right;
                    let remain = left % right;
                    if remain > 0 {
                        value + 1
                    } else {
                        value
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
                                let remain = (range % 2) as $integer;
                                let shift = (start + center).max(0) + (end - center - remain).min(0);
                                let wrap = value.wrapping_add(shift).wrapping_sub(center);
                                debug_assert!(wrap >= start && wrap <= end);
                                wrap
                            }
                        }
                        // TODO: Generate 'small' values first. Maybe use the same adjustment as Random?
                        Mode::Exhaustive(index) => consume(index, start as _, end as _) as $integer,
                    }
                }
                generate(self, range.into())
            }
        }
    };
    ($([$integer: ident, $positive: ident, $constant: ident]),*) => {
        $(integer!($integer, $positive, $constant);)*
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
                        let power = (log as f64 / scale).max(1.0);
                        let pow = size.powf(power);
                        range * pow as $number
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
                        Mode::Exhaustive(index) => utility::$number::from_bits(consume(
                            index,
                            utility::$number::to_bits(start) as _,
                            utility::$number::to_bits(end) as _) as _),
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
    [u8, u8, U8],
    [u16, u16, U16],
    [u32, u32, U32],
    [u64, u64, U64],
    [u128, u128, U128],
    [usize, usize, Usize],
    [i8, u8, I8],
    [i16, u16, I16],
    [i32, u32, I32],
    [i64, u64, I64],
    [i128, u128, I128],
    [isize, usize, Isize]
);

floating!([f32, i32], [f64, i64]);

impl Modes {
    pub(crate) fn with(
        count: usize,
        sizes: Sizes,
        seed: u64,
        cardinality: Option<u128>,
        exhaustive: Option<bool>,
    ) -> Self {
        match exhaustive {
            Some(true) => Modes::Exhaustive(count),
            Some(false) => Modes::Random { count, sizes, seed },
            None => match cardinality.map(usize::try_from) {
                Some(Ok(cardinality)) if cardinality <= count => Modes::Exhaustive(cardinality),
                _ => Modes::Random { count, sizes, seed },
            },
        }
    }

    pub(crate) fn state(self, index: usize) -> State {
        match self {
            Modes::Random { count, sizes, seed } => State::random(index, count, sizes, seed),
            Modes::Exhaustive(count) => State::exhaustive(index, count),
        }
    }
}

impl Default for Modes {
    fn default() -> Self {
        Modes::Random {
            count: GENERATES,
            sizes: Sizes::default(),
            seed: seed(),
        }
    }
}

impl From<Modes> for States {
    fn from(modes: Modes) -> Self {
        let count = match modes {
            Modes::Random { count, .. } => count,
            Modes::Exhaustive(count) => count,
        };
        States {
            indices: 0..count,
            modes,
        }
    }
}

impl Default for States {
    fn default() -> Self {
        Modes::default().into()
    }
}

impl Iterator for States {
    type Item = State;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.modes.state(self.indices.next()?))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }

    fn count(self) -> usize {
        self.indices.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        Some(self.modes.state(self.indices.nth(n)?))
    }

    fn last(self) -> Option<Self::Item> {
        Some(self.modes.state(self.indices.last()?))
    }
}

impl ExactSizeIterator for States {
    fn len(&self) -> usize {
        self.indices.len()
    }
}

impl DoubleEndedIterator for States {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some(self.modes.state(self.indices.next_back()?))
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        Some(self.modes.state(self.indices.nth_back(n)?))
    }
}

impl FusedIterator for States {}

#[cfg(feature = "parallel")]
mod parallel {
    use super::*;
    use rayon::{
        iter::{IntoParallelIterator, ParallelIterator},
        range::Iter,
    };

    pub struct Iterator(Iter<usize>, Modes);

    impl IntoParallelIterator for States {
        type Item = State;
        type Iter = Iterator;

        fn into_par_iter(self) -> Self::Iter {
            Iterator(self.indices.into_par_iter(), self.modes)
        }
    }

    impl ParallelIterator for Iterator {
        type Item = State;

        fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where
            C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
        {
            let Self(indices, modes) = self;
            indices
                .map(move |index| modes.state(index))
                .drive_unindexed(consumer)
        }

        fn opt_len(&self) -> Option<usize> {
            self.0.opt_len()
        }
    }
}

impl Sizes {
    pub(crate) const DEFAULT: Self = Self::new(0.0, 1.0, Self::SCALE);
    pub(crate) const SCALE: f64 = 6.0;

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
    fn is_within_bounds() {
        for i in 0..=100 {
            let mut state = State::random(i, 100, Sizes::DEFAULT, 0);
            let value = state.i8(-128..=1);
            assert!(value >= -128 && value <= 1);
        }
    }

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
            let mut values =
                Iterator::map(0..count * 25, |_| generate(&mut state)).collect::<Vec<_>>();
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
            let mut values = Iterator::map(0..count, |i| {
                generate(&mut State::exhaustive(i as _, count))
            })
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
