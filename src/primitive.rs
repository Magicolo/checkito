use crate::{
    any::Unify,
    generate::{FullGenerate, Generate, IntoGenerate, State},
    shrink::{FullShrink, IntoShrink, Shrink},
    Nudge,
};
use std::{
    convert::TryInto,
    marker::PhantomData,
    ops::{self, Bound},
};

#[derive(Copy, Clone, Debug, Default)]
pub struct Full<T: ?Sized>(PhantomData<T>);

#[derive(Copy, Clone, Debug, Default)]
pub struct Range<T> {
    pub start: T,
    pub end: T,
}

#[derive(Copy, Clone, Debug)]
pub enum Error {
    Overflow,
    Empty,
    Invalid,
}

#[derive(Copy, Clone, Debug)]
enum Direction {
    None,
    Low,
    High,
}

#[derive(Clone, Debug)]
pub struct Shrinker<T> {
    range: Range<T>,
    item: T,
    direction: Direction,
}

impl<T: ?Sized> Full<T> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Range<T> {
    pub fn map<U, F: FnMut(T) -> U>(self, mut map: F) -> Range<U> {
        Range {
            start: map(self.start),
            end: map(self.end),
        }
    }
}

impl<T> Shrinker<T> {
    pub const fn new(range: Range<T>, item: T) -> Self {
        Self {
            range,
            item,
            direction: Direction::None,
        }
    }

    pub fn map<U, F: FnMut(T) -> U>(self, mut map: F) -> Shrinker<U> {
        let item = map(self.item);
        Shrinker {
            range: self.range.map(map),
            item,
            direction: self.direction,
        }
    }
}

impl From<Range<char>> for Range<u32> {
    fn from(value: Range<char>) -> Self {
        value.map(|value| value as u32)
    }
}

impl From<Range<u8>> for Range<char> {
    fn from(value: Range<u8>) -> Self {
        value.map(|value| value as char)
    }
}

impl From<Shrinker<char>> for Shrinker<u32> {
    fn from(value: Shrinker<char>) -> Self {
        value.map(|value| value as u32)
    }
}

impl From<Shrinker<u8>> for Shrinker<u32> {
    fn from(value: Shrinker<u8>) -> Self {
        value.map(|value| value as u32)
    }
}

impl From<Shrinker<u8>> for Shrinker<char> {
    fn from(value: Shrinker<u8>) -> Self {
        value.map(|value| value as char)
    }
}

impl TryFrom<Range<u32>> for Range<char> {
    type Error = <char as TryFrom<u32>>::Error;

    fn try_from(value: Range<u32>) -> Result<Self, Self::Error> {
        Ok(Self {
            start: value.start.try_into()?,
            end: value.end.try_into()?,
        })
    }
}

impl TryFrom<Shrinker<u32>> for Shrinker<char> {
    type Error = <char as TryFrom<u32>>::Error;

    fn try_from(value: Shrinker<u32>) -> Result<Self, Self::Error> {
        Ok(Self {
            range: value.range.try_into()?,
            item: value.item.try_into()?,
            direction: value.direction,
        })
    }
}

impl<T> ops::RangeBounds<T> for Range<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(&self.start)
    }

    fn end_bound(&self) -> Bound<&T> {
        Bound::Included(&self.end)
    }
}

macro_rules! full {
    ($t:ty) => {
        impl FullGenerate for $t {
            type Item = $t;
            type Generate = Full<$t>;

            fn generator() -> Self::Generate {
                Full::<$t>::new()
            }
        }

        impl FullShrink for $t {
            type Item = $t;
            type Shrink = <Full<$t> as Generate>::Shrink;

            fn shrinker(item: Self::Item) -> Option<Self::Shrink> {
                Full::<$t>::new().shrinker(item)
            }
        }

        impl Generate for $t {
            type Item = Self;
            type Shrink = Self;

            fn generate(&self, _: &mut State) -> Self::Shrink {
                *self
            }
        }

        impl Shrink for $t {
            type Item = Self;

            fn item(&self) -> Self::Item {
                *self
            }

            fn shrink(&mut self) -> Option<Self> {
                None
            }
        }
    };
}

macro_rules! range {
    ($i:ident, $t:ty, $r:ty) => {
        impl TryFrom<$r> for $t {
            type Error = Error;

            fn try_from(range: $r) -> Result<Self, Self::Error> {
                Self::$i(range)
            }
        }

        impl IntoGenerate for $r {
            type Item = $i;
            type Generate = $t;
            fn generator(self) -> Self::Generate {
                <$t as TryFrom<$r>>::try_from(self).unwrap()
            }
        }

        impl Generate for $r {
            type Item = $i;
            type Shrink = <$t as Generate>::Shrink;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                <$t as TryFrom<$r>>::try_from(self.clone())
                    .unwrap()
                    .generate(state)
            }
        }

        impl IntoShrink for $r {
            type Item = $i;
            type Shrink = <$t as Generate>::Shrink;

            fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
                <$t as TryFrom<$r>>::try_from(self.clone())
                    .ok()?
                    .shrinker(item)
            }
        }
    };
}

macro_rules! ranges {
    ($i:ident, $t:ty) => {
        impl TryFrom<ops::RangeFull> for $t {
            type Error = Error;

            fn try_from(range: ops::RangeFull) -> Result<Self, Self::Error> {
                Self::$i(range)
            }
        }

        range!($i, $t, ops::Range<$i>);
        range!($i, $t, ops::RangeInclusive<$i>);
        range!($i, $t, ops::RangeFrom<$i>);
        range!($i, $t, ops::RangeTo<$i>);
        range!($i, $t, ops::RangeToInclusive<$i>);
    };
}

macro_rules! shrinked {
    ($t:ident) => {
        impl Range<$t> {
            pub(super) fn shrinked(&self, size: f64) -> Self {
                fn shrink(range: f64, size: f64) -> f64 {
                    // This adjustment of the size tries to prevent large ranges (such as `u64`) from rushing into huge
                    // values as soon as the `size > 0`.
                    let power = range.abs().log2() / 8.0;
                    if power < 1.0 {
                        range * size
                    } else {
                        range * size.powf(power)
                    }
                }

                if self.start >= 0 as $t {
                    debug_assert!(self.end >= 0 as $t);
                    let range = (self.end - self.start) as f64;
                    let shrunk = shrink(range, size);
                    let end = (self.start as f64 + shrunk) as $t;
                    Self {
                        start: self.start,
                        end: end.max(self.start).min(self.end),
                    }
                } else if self.end <= 0 as $t {
                    debug_assert!(self.start <= 0 as $t);
                    let range = (self.start - self.end) as f64;
                    let shrunk = shrink(range, size);
                    let start = (self.end as f64 + shrunk) as $t;
                    Self {
                        start: start.min(self.end).max(self.start),
                        end: self.end,
                    }
                } else {
                    debug_assert!(self.start < 0 as $t);
                    debug_assert!(self.end > 0 as $t);

                    let start = self.start as f64;
                    let end = self.end as f64;
                    let left = shrink(start, size) * 0.5;
                    let right = shrink(end, size) * 0.5;
                    let mut ranges = (left - right, right - left);
                    if ranges.0 < start {
                        ranges.1 += start - ranges.0;
                    } else if ranges.1 > end {
                        ranges.0 += end - ranges.1;
                    }
                    Self {
                        start: (ranges.0 as $t).min(self.end).max(self.start),
                        end: (ranges.1 as $t).max(self.start).min(self.end),
                    }
                }
            }
        }
    };
}

macro_rules! shrink {
    ($s:expr, $t:ident) => {{
        let target = match $s.direction {
            Direction::None if $s.item >= 0 as $t => {
                $s.range.end = $s.item;
                $s.item = $s.range.start.max(0 as $t);
                $s.direction = Direction::High;
                $s.range.end
            }
            Direction::None => {
                $s.range.start = $s.item;
                $s.item = $s.range.end.min(0 as $t);
                $s.direction = Direction::Low;
                $s.range.start
            }
            Direction::Low => $s.range.start,
            Direction::High => $s.range.end,
        };

        let old = $s.item;
        // Divide both sides of the division by 2 to prevent overflows.
        let delta = target / 2 as $t - old / 2 as $t;
        let new = old + delta;
        if old == new {
            None
        } else {
            $s.item = new;
            Some(Shrinker {
                direction: Direction::None,
                range: $s.range,
                item: old,
            })
        }
    }};
}

pub mod boolean {
    use super::*;

    #[derive(Copy, Clone, Debug, Default)]
    pub struct Shrinker(bool);

    impl Generate for Full<bool> {
        type Item = bool;
        type Shrink = Shrinker;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrinker(state.random().f64() * state.size() >= 0.5)
        }
    }

    impl IntoShrink for Full<bool> {
        type Item = bool;
        type Shrink = Shrinker;

        fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
            Some(Shrinker(item))
        }
    }

    impl Shrink for Shrinker {
        type Item = bool;

        fn item(&self) -> Self::Item {
            self.0
        }

        fn shrink(&mut self) -> Option<Self> {
            if self.0 {
                self.0 = false;
                Some(*self)
            } else {
                None
            }
        }
    }

    full!(bool);
}

pub mod character {
    use super::*;

    #[derive(Copy, Clone, Debug, Default)]
    pub struct Range(super::Range<u32>);
    #[derive(Clone, Debug)]
    pub struct Shrinker(super::Shrinker<u32>);

    impl Range {
        pub fn char(range: impl ops::RangeBounds<char>) -> Result<Self, Error> {
            let start = match range.start_bound() {
                Bound::Included(&bound) => bound as u32,
                Bound::Excluded(&bound) => (bound as u32).checked_add(1).ok_or(Error::Overflow)?,
                Bound::Unbounded => 0 as u32,
            };
            let end = match range.end_bound() {
                Bound::Included(&bound) => bound as u32,
                Bound::Excluded(&bound) => (bound as u32).checked_sub(1).ok_or(Error::Overflow)?,
                Bound::Unbounded => char::MAX as u32,
            };
            if end < start {
                Err(Error::Empty)
            } else {
                Ok(Self(super::Range { start, end }))
            }
        }
    }

    impl Full<char> {
        const fn range() -> Range {
            Range(super::Range {
                start: 0,
                end: char::MAX as u32,
            })
        }

        const fn shrink(item: char) -> Shrinker {
            Shrinker(super::Shrinker::new(Self::range().0, item as u32))
        }

        const fn special<'a>() -> impl Generate<Item = char, Shrink = char> {
            struct Special;
            impl Generate for Special {
                type Item = char;
                type Shrink = char;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    (
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
                    )
                        .any()
                        .generate(state)
                        .unify()
                }
            }
            Special
        }
    }

    impl From<super::Shrinker<u8>> for Shrinker {
        fn from(value: super::Shrinker<u8>) -> Self {
            Self(value.into())
        }
    }

    impl From<super::Shrinker<char>> for Shrinker {
        fn from(value: super::Shrinker<char>) -> Self {
            Self(value.into())
        }
    }

    impl Generate for Range {
        type Item = char;
        type Shrink = Shrinker;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrinker(self.0.generate(state))
        }
    }

    impl Generate for Full<char> {
        type Item = char;
        type Shrink = Shrinker;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            match state.random().u8(..) {
                ..=253 => Self::range().generate(state),
                254.. => Self::shrink(Self::special().generate(state)),
            }
        }
    }

    impl IntoShrink for Range {
        type Item = char;
        type Shrink = Shrinker;

        fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
            Some(Shrinker(self.0.shrinker(item as u32)?))
        }
    }

    impl IntoShrink for Full<char> {
        type Item = char;
        type Shrink = Shrinker;

        fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
            Self::range().shrinker(item)
        }
    }

    impl Shrink for Shrinker {
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
    ranges!(char, Range);
}

pub mod number {
    use super::*;

    macro_rules! integer {
        ($t:ident) => {
            impl Full<$t> {
                const fn range() -> Range<$t> {
                    Range { start: $t::MIN, end: $t::MAX }
                }

                const fn shrink(item: $t) -> Shrinker<$t> {
                    Shrinker::new(Self::range(), item)
                }

                const fn special<'a>() -> impl Generate<Item = $t, Shrink = $t> {
                    struct Special;
                    impl Generate for Special {
                        type Item = $t;
                        type Shrink = $t;

                        fn generate(&self, state: &mut State) -> Self::Shrink {
                            (0 as $t, $t::MIN, $t::MAX).any().generate(state).unify()
                        }
                    }
                    Special
                }
            }

            impl Range<$t> {
                pub fn $t(range: impl ops::RangeBounds<$t>) -> Result<Self, Error> {
                    let start = match range.start_bound() {
                        Bound::Included(&bound) => bound,
                        Bound::Excluded(&bound) => bound.checked_add(1 as $t).ok_or(Error::Overflow)?,
                        Bound::Unbounded => $t::MIN,
                    };
                    let end = match range.end_bound() {
                        Bound::Included(&bound) => bound,
                        Bound::Excluded(&bound) => bound.checked_sub(1 as $t).ok_or(Error::Overflow)?,
                        Bound::Unbounded => $t::MAX,
                    };
                    if end < start {
                        Err(Error::Empty)
                    } else {
                        Ok(Self { start, end })
                    }
                }
            }
            shrinked!($t);

            impl Generate for Range<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    let range = self.shrinked(state.size());
                    let item = state.random().$t(range.start..=range.end);
                    Shrinker::new(range, item)
                }
            }

            impl Generate for Full<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    match state.random().u8(..) {
                        0..=254 => Self::range().shrinked(state.size()).generate(state),
                        255 => Self::shrink(Self::special().generate(state)),
                    }
                }
            }

            impl IntoShrink for Range<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
                    if item >= self.start && item <= self.end {
                        Some(Shrinker::new(self.clone(), item))
                    } else {
                        None
                    }
                }
            }

            impl IntoShrink for Full<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
                    Some(Self::shrink(item))
                }
            }

            impl Shrink for Shrinker<$t> {
                type Item = $t;

                fn item(&self) -> Self::Item {
                    self.item
                }

                fn shrink(&mut self) -> Option<Self> {
                    shrink!(self, $t)
                }
            }

            full!($t);
            ranges!($t, Range<$t>);
        };
        ($($ts:ident),*) => { $(integer!($ts);)* };
    }

    macro_rules! floating {
        ($t:ident) => {
            impl Full<$t> {
                const fn special<'a>() -> impl Generate<Item = $t, Shrink = $t> {
                    struct Special;
                    impl Generate for Special {
                        type Item = $t;
                        type Shrink = $t;

                        fn generate(&self, state: &mut State) -> Self::Shrink {
                            (0 as $t, $t::MIN, $t::MAX, $t::EPSILON, $t::INFINITY, $t::NEG_INFINITY, $t::MIN_POSITIVE, $t::NAN)
                                .any()
                                .generate(state)
                                .unify()
                        }
                    }
                    Special
                }

                const fn range() -> Range<$t> {
                    Range {
                        start: $t::MIN,
                        end: $t::MAX,
                    }
                }

                fn sub_range() -> Range<$t> {
                    Range {
                        start: -1 as $t / $t::EPSILON,
                        end: 1 as $t / $t::EPSILON
                    }
                }

                const fn shrink(item: $t) -> Shrinker<$t> {
                    Shrinker::new(Self::range(), item)
                }
            }

            impl Range<$t> {
                pub fn $t(range: impl ops::RangeBounds<$t>) -> Result<Self, Error> {
                    let start = match range.start_bound() {
                        Bound::Included(&bound) => (bound, false),
                        Bound::Excluded(&bound) => (bound, true),
                        Bound::Unbounded => ($t::MIN, false),
                    };
                    let end = match range.end_bound() {
                        Bound::Included(&bound) => (bound, false),
                        Bound::Excluded(&bound) => (bound, true),
                        Bound::Unbounded => ($t::MAX, false),
                    };

                    if end.0 < start.0 {
                        Err(Error::Empty)
                    } else if (start.1 || end.1) && start.0 == end.0 {
                        Err(Error::Empty)
                    } else {
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
                        Ok(Self {
                            start: start.min(end),
                            end: end.max(start),
                        })
                    }
                }
            }
            shrinked!($t);

            impl Generate for Range<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    let range = self.shrinked(state.size());
                    let ratio = state.random().$t();
                    let difference = range.end * ratio - range.start * ratio;
                    let item = (difference + range.start).max(range.start).min(range.end);
                    Shrinker::new(range, item)
                }
            }

            impl Generate for Full<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    match state.random().u8(..) {
                        ..=93 => Self::sub_range().shrinked(state.size()).generate(state),
                        94..=187 => Self::sub_range().shrinked(state.size()).map(|value| 1 as $t / value).generate(state),
                        188..=219 => Self::range().shrinked(state.size()).generate(state),
                        220..=251 => Self::range().shrinked(state.size()).map(|value| 1 as $t / value).generate(state),
                        252.. => Self::shrink(Self::special().generate(state)),
                    }
                }
            }

            impl IntoShrink for Range<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
                    if item >= self.start && item <= self.end {
                        Some(Shrinker::new(self.clone(), item))
                    } else {
                        None
                    }
                }
            }

            impl IntoShrink for Full<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
                    Some(Self::shrink(item))
                }
            }

            impl Shrink for Shrinker<$t> {
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

            full!($t);
            ranges!($t, Range<$t>);
        };
        ($($ts:ident),*) => { $(floating!($ts);)* };
    }

    integer!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
    floating!(f32, f64);
}
