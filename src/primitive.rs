use crate::{
    any::Any,
    generate::{FullGenerator, Generator, State},
    nudge::Nudge,
    shrink::Shrinker,
};
use core::{
    convert::TryInto,
    marker::PhantomData,
    ops::{self, Bound},
};

#[derive(Copy, Clone, Debug)]
pub(crate) enum Direction {
    None,
    Low,
    High,
}

#[derive(Debug)]
pub struct Full<T: ?Sized>(PhantomData<T>);

#[derive(Debug)]
pub struct Special<T: ?Sized>(PhantomData<T>);

#[derive(Clone, Debug)]
pub struct Shrink<T> {
    pub(crate) start: T,
    pub(crate) end: T,
    pub(crate) item: T,
    pub(crate) direction: Direction,
}

impl<T: ?Sized> Special<T> {
    pub(crate) const NEW: Self = Self(PhantomData);
}

impl<T: ?Sized> Clone for Special<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: ?Sized> Copy for Special<T> {}

impl<T: ?Sized> Full<T> {
    pub(crate) const NEW: Self = Self(PhantomData);
}

impl<T: ?Sized> Clone for Full<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: ?Sized> Copy for Full<T> {}

macro_rules! full {
    ($t:ty) => {
        impl FullGenerator for $t {
            type FullGen = Full<$t>;
            type Item = $t;

            fn full_gen() -> Self::FullGen {
                Full::<$t>::NEW
            }
        }
    };
}

macro_rules! same {
    ($t:ty) => {
        impl Generator for $t {
            type Item = Self;
            type Shrink = Self;

            fn generate(&self, _: &mut State) -> Self::Shrink {
                <$t as Clone>::clone(self)
            }

            fn constant(&self) -> bool {
                true
            }
        }

        impl Shrinker for $t {
            type Item = Self;

            fn item(&self) -> Self::Item {
                <$t as Clone>::clone(self)
            }

            fn shrink(&mut self) -> Option<Self> {
                None
            }
        }
    };
}

macro_rules! range {
    (CHARACTER, $t:ident, $r:ty) => {
        impl Generator for $r {
            type Item = $t;
            type Shrink = Shrink;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                let (start, end) = range(self);
                Shrink((start..=end).generate(state))
            }

            fn constant(&self) -> bool {
                let (start, end) = range(self);
                (start..=end).constant()
            }
        }
    };
    (INTEGER, $t:ident, $r:ty) => {
        impl Generator for $r {
            type Item = $t;
            type Shrink = Shrink<$t>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                let (start, end) = range(self);
                let (start, end) = shrinked((start, end), state.size());
                let item = state.random().$t(start..=end);
                Shrink {
                    start,
                    end,
                    item,
                    direction: Direction::None,
                }
            }

            fn constant(&self) -> bool {
                let (start, end) = range(self);
                start == end
            }
        }
    };
    (FLOATING, $t:ident, $r:ty) => {
        impl Generator for $r {
            type Item = $t;
            type Shrink = Shrink<$t>;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                let (start, end) = range(self);
                debug_assert!(start.is_finite() && end.is_finite());
                let (start, end) = shrinked((start, end), state.size());
                debug_assert!(start.is_finite() && end.is_finite());
                let ratio = state.random().$t();
                debug_assert!(ratio.is_finite() && ratio >= 0 as $t && ratio <= 1 as $t);
                let difference = end * ratio - start * ratio;
                let item = (difference + start).clamp(start, end);
                debug_assert!(item.is_finite());
                Shrink {
                    start,
                    end,
                    item,
                    direction: Direction::None,
                }
            }

            fn constant(&self) -> bool {
                let (start, end) = range(self);
                start == end
            }
        }
    };
}

macro_rules! ranges {
    ($k: ident, $t:ident) => {
        range!($k, $t, ops::Range<$t>);
        range!($k, $t, ops::RangeInclusive<$t>);
        range!($k, $t, ops::RangeFrom<$t>);
        range!($k, $t, ops::RangeTo<$t>);
        range!($k, $t, ops::RangeToInclusive<$t>);
    };
}

macro_rules! shrinked {
    ($t:ident) => {
        pub(crate) fn shrinked(pair: ($t, $t), size: f64) -> ($t, $t) {
            fn shrink(range: f64, size: f64) -> f64 {
                // This adjustment of the size tries to prevent large ranges (such as `u64`)
                // from rushing into huge values as soon as the `size > 0`.
                range * size.powf(range.abs().log2() / 12.0)
            }

            if pair.0 >= 0 as $t {
                debug_assert!(pair.1 >= 0 as $t);
                let range = (pair.1 - pair.0) as f64;
                let shrunk = shrink(range, size);
                let end = (pair.0 as f64 + shrunk) as $t;
                (pair.0, end.clamp(pair.0, pair.1))
            } else if pair.1 <= 0 as $t {
                debug_assert!(pair.0 <= 0 as $t);
                let range = (pair.0 - pair.1) as f64;
                let shrunk = shrink(range, size);
                let start = (pair.1 as f64 + shrunk) as $t;
                (start.clamp(pair.0, pair.1), pair.1)
            } else {
                debug_assert!(pair.0 < 0 as $t);
                debug_assert!(pair.1 > 0 as $t);
                let start = pair.0 as f64;
                let end = pair.1 as f64;
                let left = shrink(start, size) * 0.5;
                let right = shrink(end, size) * 0.5;
                let mut ranges = (left - right, right - left);
                if ranges.0 < start {
                    ranges.1 += start - ranges.0;
                } else if ranges.1 > end {
                    ranges.0 += end - ranges.1;
                }
                (
                    (ranges.0 as $t).clamp(pair.0, pair.1),
                    (ranges.1 as $t).clamp(pair.0, pair.1),
                )
            }
        }
    };
}

macro_rules! shrink {
    ($s:expr, $t:ident) => {{
        // Never change `$s.item` to preserve coherence in calls to `shrinker.item()`.
        match $s.direction {
            Direction::None if $s.item >= 0 as $t => {
                $s.start = $s.start.max(0 as $t);
                if $s.start == $s.item {
                    None
                } else {
                    $s.direction = Direction::High;
                    $s.end = $s.item;
                    Some(Shrink {
                        direction: $s.direction,
                        start: $s.start,
                        end: $s.start,
                        item: $s.start,
                    })
                }
            }
            Direction::None => {
                $s.end = $s.end.min(0 as $t);
                if $s.end == $s.item {
                    None
                } else {
                    $s.direction = Direction::Low;
                    $s.start = $s.item;
                    Some(Shrink {
                        direction: $s.direction,
                        start: $s.end,
                        end: $s.end,
                        item: $s.end,
                    })
                }
            }
            Direction::Low => {
                let delta = $s.end / 2 as $t - $s.start / 2 as $t;
                let middle = $s.start + delta;
                if middle == $s.start || middle == $s.end {
                    None
                } else {
                    let mut shrinker = $s.clone();
                    shrinker.start = middle;
                    shrinker.item = middle;
                    $s.end = middle;
                    Some(shrinker)
                }
            }
            Direction::High => {
                let delta = $s.end / 2 as $t - $s.start / 2 as $t;
                let middle = $s.start + delta;
                if middle == $s.start || middle == $s.end {
                    None
                } else {
                    let mut shrinker = $s.clone();
                    shrinker.end = middle;
                    shrinker.item = middle;
                    $s.start = middle;
                    Some(shrinker)
                }
            }
        }
    }};
}

pub mod bool {
    use super::*;
    use core::mem::take;

    #[derive(Copy, Clone, Debug)]
    pub struct Shrink(bool, bool);

    impl Generator for Full<bool> {
        type Item = bool;
        type Shrink = Shrink;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrink(true, state.random().bool())
        }

        fn constant(&self) -> bool {
            false
        }
    }

    impl Shrinker for Shrink {
        type Item = bool;

        fn item(&self) -> Self::Item {
            self.1
        }

        fn shrink(&mut self) -> Option<Self> {
            // A distinct `bool` is required to avoid modifying the `item()` if it would be
            // called after shrink.
            if self.1 && take(&mut self.0) {
                Some(Shrink(false, false))
            } else {
                None
            }
        }
    }

    full!(bool);
    same!(bool);
}

pub mod char {
    use super::*;

    #[derive(Clone, Debug)]
    pub struct Shrink(super::Shrink<u32>);

    impl Generator for Special<char> {
        type Item = char;
        type Shrink = char;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Any((
                '\\',
                '\x0B',
                '\x1B',
                '\x7F',
                '\u{0000}',
                '\u{D7FF}',
                '\u{E000}',
                '\u{FEFF}',
                '\u{202E}',
                '¥',
                'Ѩ',
                'Ⱥ',
                '🕴',
                char::MAX,
                char::REPLACEMENT_CHARACTER,
            ))
            .generate(state)
            .into()
        }

        fn constant(&self) -> bool {
            false
        }
    }

    fn range<R: ops::RangeBounds<char>>(range: &R) -> (u32, u32) {
        let start = match range.start_bound() {
            Bound::Included(&bound) => Bound::Included(bound as u32),
            Bound::Excluded(&bound) => Bound::Excluded(bound as u32),
            Bound::Unbounded => Bound::Included(0),
        };
        let end = match range.end_bound() {
            Bound::Included(&bound) => Bound::Included(bound as u32),
            Bound::Excluded(&bound) => Bound::Excluded(bound as u32),
            Bound::Unbounded => Bound::Included(char::MAX as u32),
        };
        number::u32::range(&(start, end))
    }

    pub(crate) const fn shrink(item: char) -> Shrink {
        Shrink(number::u32::shrink(item as u32))
    }

    impl Generator for Full<char> {
        type Item = char;
        type Shrink = Shrink;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            match state.random().u8(..) {
                0..250 => (0 as char..=char::MAX).generate(state),
                250.. => shrink(Special::<char>::NEW.generate(state)),
            }
        }

        fn constant(&self) -> bool {
            false
        }
    }

    impl Shrinker for Shrink {
        type Item = char;

        fn item(&self) -> Self::Item {
            self.0
                .item()
                .try_into()
                .unwrap_or(char::REPLACEMENT_CHARACTER)
        }

        fn shrink(&mut self) -> Option<Self> {
            Some(Self(self.0.shrink()?))
        }
    }

    full!(char);
    same!(char);
    ranges!(CHARACTER, char);
}

pub mod string {
    use super::*;

    same!(&str);
    same!(Box<str>);
    same!(String);
}

pub mod number {
    use super::*;

    pub trait Number: Sized {
        type Full: Generator<Item = Self>;
        type Special: Generator<Item = Self>;
        type Positive: Generator<Item = Self>;
        type Negative: Generator<Item = Self>;

        const ZERO: Self;
        const ONE: Self;
        const MIN: Self;
        const MAX: Self;
        const FULL: Self::Full;
        const SPECIAL: Self::Special;
        const POSITIVE: Self::Positive;
        const NEGATIVE: Self::Negative;
    }

    macro_rules! number {
        ($t: ident) => {
            impl Number for $t {
                type Full = ops::RangeInclusive<Self>;
                type Negative = ops::RangeInclusive<Self>;
                type Positive = ops::RangeInclusive<Self>;
                type Special = Special<Self>;

                const FULL: Self::Full = Self::MIN..=Self::MAX;
                const MAX: Self = $t::MAX;
                const MIN: Self = $t::MIN;
                const NEGATIVE: Self::Negative = Self::MIN..=Self::ZERO;
                const ONE: Self = 1 as $t;
                const POSITIVE: Self::Positive = Self::ZERO..=Self::MAX;
                const SPECIAL: Self::Special = Special::<$t>::NEW;
                const ZERO: Self = 0 as $t;
            }
        };
    }

    macro_rules! integer {
        ($t:ident) => {
            impl Generator for Special<$t> {
                type Item = $t;
                type Shrink = $t;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    Any((0 as $t, $t::MIN, $t::MAX)).generate(state).into()
                }

                fn constant(&self) -> bool {
                    false
                }
            }

            pub(crate) const fn shrink(item: $t) -> Shrink<$t> {
                Shrink { start: item, end: item, item, direction: Direction::None }
            }

            impl Generator for Full<$t> {
                type Item = $t;
                type Shrink = Shrink<$t>;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    match state.random().u8(..) {
                        0..250 => ($t::MIN..=$t::MAX).generate(state),
                        250.. => shrink(Special::<$t>::NEW.generate(state)),
                    }
                }

                fn constant(&self) -> bool {
                    false
                }
            }

            impl Shrinker for Shrink<$t> {
                type Item = $t;

                fn item(&self) -> Self::Item {
                    self.item
                }

                fn shrink(&mut self) -> Option<Self> {
                    shrink!(self, $t)
                }
            }

            shrinked!($t);

            /// - An empty range (0..=0) or invalid range (0..0) will use the `start` value.
            /// - An reversed range will be flipped.
            pub fn range<R: ops::RangeBounds<$t>>(range: &R) -> ($t, $t) {
                let mut start = match range.start_bound() {
                    Bound::Included(&bound) => (bound, false),
                    Bound::Excluded(&bound) => (bound, true),
                    Bound::Unbounded => ($t::MIN, false),
                };
                let mut end = match range.end_bound() {
                    Bound::Included(&bound) => (bound, false),
                    Bound::Excluded(&bound) => (bound, true),
                    Bound::Unbounded => ($t::MAX, false),
                };
                if start.0 == end.0 {
                    return (start.0, end.0);
                }
                if start.0 > end.0 {
                    (start, end) = (end, start);
                }
                if start.1 {
                    start.0 = start.0.saturating_add(1 as $t);
                }
                if end.1 {
                    end.0 = end.0.saturating_sub(1 as $t);
                }
                (start.0.clamp($t::MIN, end.0), end.0.clamp(start.0, $t::MAX))
            }

            full!($t);
            same!($t);
            ranges!(INTEGER, $t);
            number!($t);
        };
        ($($ts:ident),*) => { $(pub(crate) mod $ts { use super::*; integer!($ts); })* };
    }

    macro_rules! floating {
        ($t:ident) => {
            impl Generator for Special<$t> {
                type Item = $t;
                type Shrink = $t;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    Any((0 as $t, $t::MIN, $t::MAX, $t::EPSILON, $t::INFINITY, $t::NEG_INFINITY, $t::MIN_POSITIVE, $t::NAN))
                        .generate(state)
                        .into()
                }

                fn constant(&self) -> bool {
                    false
                }
            }

            pub(crate) const fn shrink(item: $t) -> Shrink<$t> {
                Shrink { start: item, end: item, item, direction: Direction::None }
            }

            shrinked!($t);

            impl Generator for Full<$t> {
                type Item = $t;
                type Shrink = Shrink<$t>;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    match state.random().u8(..) {
                        0..90 => ($t::MIN..=$t::MAX).generate(state),
                        90..180 => (-1 as $t / $t::EPSILON..=1 as $t / $t::EPSILON).generate(state),
                        180..215 => (1 as $t / $t::MIN..=1 as $t / $t::MAX).generate(state),
                        215..250 => (-1 as $t / $t::EPSILON..=1 as $t / $t::EPSILON).generate(state),
                        250.. => shrink(Special::<$t>::NEW.generate(state)),
                    }
                }

                fn constant(&self) -> bool {
                    false
                }
            }

            impl Shrinker for Shrink<$t> {
                type Item = $t;

                fn item(&self) -> Self::Item {
                    self.item
                }

                fn shrink(&mut self) -> Option<Self> {
                    if self.item.is_finite() {
                        shrink!(self, $t)
                    } else {
                        None
                    }
                }
            }

            pub(crate) fn range<R: ops::RangeBounds<$t>>(range: &R) -> ($t, $t) {
                let mut start = match range.start_bound() {
                    Bound::Included(&bound) => (bound, false),
                    Bound::Excluded(&bound) => (bound, true),
                    Bound::Unbounded => ($t::MIN, false),
                };
                let mut end = match range.end_bound() {
                    Bound::Included(&bound) => (bound, false),
                    Bound::Excluded(&bound) => (bound, true),
                    Bound::Unbounded => ($t::MAX, false),
                };
                assert!(start.0.is_finite());
                assert!(end.0.is_finite());

                if start.0 == end.0 {
                    return (start.0, end.0);
                }
                if start.0 > end.0 {
                    (start, end) = (end, start);
                }

                let start = if start.1 {
                    start.0.nudge(start.0.signum())
                } else {
                    start.0
                };
                let end = if end.1 {
                    end.0.nudge(-end.0.signum())
                } else {
                    end.0
                };
                // `Nudge` can push a value to infinity, so clamp brings it back in valid range.
                (start.clamp($t::MIN, end), end.clamp(start, $t::MAX))
            }

            full!($t);
            same!($t);
            ranges!(FLOATING, $t);
            number!($t);
        };
        ($($ts:ident),*) => { $(pub mod $ts { use super::*; floating!($ts); })* };
    }

    integer!(
        u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
    );
    floating!(f32, f64);
}
