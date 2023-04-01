use crate::{
    generate::{FullGenerate, Generate, IntoGenerate, State},
    shrink::Shrink,
    Nudge,
};
use std::{
    convert::TryInto,
    marker::PhantomData,
    mem::size_of,
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

macro_rules! constant {
    ($t:ty) => {
        impl Generate for $t {
            type Item = Self;
            type Shrink = Self;

            fn generate(&self, _: &mut State) -> (Self::Item, Self::Shrink) {
                (*self, *self)
            }
        }

        impl Shrink for $t {
            type Item = Self;

            fn generate(&self) -> Self::Item {
                *self
            }

            fn shrink(&mut self) -> Option<Self> {
                None
            }
        }
    };
}

macro_rules! range {
    ($t:ty, $r:ty) => {
        impl TryFrom<$r> for Range<$t> {
            type Error = Error;

            fn try_from(range: $r) -> Result<Self, Self::Error> {
                Range::<$t>::new(range)
            }
        }

        impl IntoGenerate for $r {
            type Item = $t;
            type Generate = Range<$t>;
            fn generator(self) -> Self::Generate {
                self.try_into().unwrap()
            }
        }

        impl Generate for $r {
            type Item = <Range<$t> as Generate>::Item;
            type Shrink = <Range<$t> as Generate>::Shrink;

            fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                self.clone().generator().generate(state)
            }
        }
    };
}

macro_rules! ranges {
    ($t:ident) => {
        impl FullGenerate for $t {
            type Item = $t;
            type Generate = Full<$t>;
            fn generator() -> Self::Generate {
                Full(PhantomData)
            }
        }

        impl TryFrom<ops::RangeFull> for Range<$t> {
            type Error = Error;

            fn try_from(range: ops::RangeFull) -> Result<Self, Self::Error> {
                Range::<$t>::new(range)
            }
        }

        range!($t, ops::Range<$t>);
        range!($t, ops::RangeInclusive<$t>);
        range!($t, ops::RangeFrom<$t>);
        range!($t, ops::RangeTo<$t>);
        range!($t, ops::RangeToInclusive<$t>);
    };
}

macro_rules! shrinked {
    ($t:ident) => {
        impl Range<$t> {
            pub(super) fn shrinked(&self, size: f64) -> Self {
                if self.start >= 0 as $t {
                    debug_assert!(self.end >= 0 as $t);
                    let range = (self.end - self.start) as f64 * size;
                    let end = (self.start as f64 + range) as $t;
                    Self {
                        start: self.start,
                        end: end.min(self.end),
                    }
                } else if self.end <= 0 as $t {
                    debug_assert!(self.start <= 0 as $t);
                    let range = (self.start - self.end) as f64 * size;
                    let start = (self.end as f64 + range) as $t;
                    Self {
                        start: start.max(self.start),
                        end: self.end,
                    }
                } else {
                    debug_assert!(self.start < 0 as $t);
                    debug_assert!(self.end > 0 as $t);

                    let start = self.start as f64;
                    let end = self.end as f64;
                    let left = start * size * 0.5;
                    let right = end * size * 0.5;
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

    impl FullGenerate for bool {
        type Item = Self;
        type Generate = Full<bool>;
        fn generator() -> Self::Generate {
            Full::new()
        }
    }

    impl Generate for Full<bool> {
        type Item = bool;
        type Shrink = Shrinker;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
            let item = state.random().f64() * state.size() >= 0.5;
            (item, Shrinker(item))
        }
    }

    impl Shrink for Shrinker {
        type Item = bool;

        fn generate(&self) -> Self::Item {
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

    constant!(bool);
}

pub mod character {
    use super::*;

    #[derive(Clone, Debug)]
    pub struct Shrinker(super::Shrinker<u32>);

    impl Range<char> {
        pub fn new(range: impl ops::RangeBounds<char>) -> Result<Self, Error> {
            let start = match range.start_bound() {
                Bound::Included(&bound) => bound,
                Bound::Excluded(&bound) => (bound as u32)
                    .checked_add(1)
                    .ok_or(Error::Overflow)?
                    .try_into()
                    .map_err(|_| Error::Invalid)?,
                Bound::Unbounded => '\u{0000}',
            };
            let end = match range.end_bound() {
                Bound::Included(&bound) => bound,
                Bound::Excluded(&bound) => (bound as u32)
                    .checked_sub(1)
                    .ok_or(Error::Overflow)?
                    .try_into()
                    .map_err(|_| Error::Invalid)?,
                Bound::Unbounded if start <= '\u{D7FF}' => '\u{D7FF}',
                Bound::Unbounded if start >= '\u{E000}' => char::MAX,
                Bound::Unbounded => return Err(Error::Invalid),
            };
            if end < start {
                Err(Error::Empty)
            } else {
                Ok(Self { start, end })
            }
        }
    }

    impl Full<char> {
        const fn low_range() -> Range<char> {
            Range {
                start: '\u{0000}',
                end: '\u{D7FF}',
            }
        }

        const fn high_range() -> Range<char> {
            Range {
                start: '\u{E000}',
                end: char::MAX,
            }
        }

        fn shrink(item: char) -> Shrinker {
            let low = Self::low_range();
            let range = if item <= low.end {
                low
            } else {
                Self::high_range()
            };
            Shrinker(super::Shrinker::new(range.into(), item as u32))
        }

        const fn special() -> impl Generate<Item = char> {
            struct Special;
            impl Generate for Special {
                type Item = char;
                type Shrink = char;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    let (item, shrink) = ('\u{0000}', char::MAX, char::REPLACEMENT_CHARACTER)
                        .any()
                        .generate(state);
                    (item.fuse(), shrink.fuse())
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

    impl Shrinker {
        pub fn new(item: char) -> Self {
            let mut low = Full::<char>::low_range();
            let range = if item <= low.end {
                low.end = item;
                low
            } else {
                let mut high = Full::<char>::high_range();
                high.start = item;
                high
            };
            Self(super::Shrinker::new(range.into(), item as u32))
        }
    }

    impl Generate for Range<char> {
        type Item = char;
        type Shrink = Shrinker;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
            let (item, shrink) = Into::<Range<u32>>::into(*self).generate(state);
            (item.try_into().unwrap(), Shrinker(shrink))
        }
    }

    impl Generate for Full<char> {
        type Item = char;
        type Shrink = Shrinker;

        fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
            fn range(range: Range<char>, size: f64, state: &mut State) -> (char, Shrinker) {
                let (item, shrink) = Into::<Range<u32>>::into(range)
                    .shrinked(size.powi(size_of::<char>() as i32))
                    .generate(state);
                (item.try_into().unwrap(), Shrinker(shrink))
            }

            match state.random().u8(..) {
                0..=250 => range(Full::<char>::low_range(), state.size(), state),
                251..=254 => range(Full::<char>::high_range(), state.size(), state),
                255 => {
                    let (item, _) = Full::<char>::special().generate(state);
                    (item, Full::<char>::shrink(item))
                }
            }
        }
    }

    impl Shrink for Shrinker {
        type Item = char;

        fn generate(&self) -> Self::Item {
            self.0.generate().try_into().unwrap()
        }

        fn shrink(&mut self) -> Option<Self> {
            Some(Self(self.0.shrink()?))
        }
    }

    constant!(char);
    ranges!(char);
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

                const fn special() -> impl Generate<Item = $t> {
                    struct Special;
                    impl Generate for Special {
                        type Item = $t;
                        type Shrink = $t;

                        fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                            let (item, shrink) = (0 as $t, $t::MIN, $t::MAX).any().generate(state);
                            (item.fuse(), shrink.fuse())
                        }
                    }
                    Special
                }
            }

            impl Range<$t> {
                pub fn new(range: impl ops::RangeBounds<$t>) -> Result<Self, Error> {
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

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    let range = self.shrinked(state.size());
                    let item = state.random().$t(range.start..=range.end);
                    (item, Shrinker::new(range, item))
                }
            }

            impl Shrink for Shrinker<$t> {
                type Item = $t;

                fn generate(&self) -> Self::Item {
                    self.item
                }

                fn shrink(&mut self) -> Option<Self> {
                    shrink!(self, $t)
                }
            }

            impl Generate for Full<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    match state.random().u8(..) {
                        0..=254 => Full::<$t>::range().shrinked(state.size().powi(size_of::<$t>() as i32)).generate(state),
                        255 => { let (item, _) = Full::<$t>::special().generate(state); (item, Full::<$t>::shrink(item)) },
                    }
                }
            }

            constant!($t);
            ranges!($t);
        };
        ($($ts:ident),*) => { $(integer!($ts);)* };
    }

    macro_rules! floating {
        ($t:ident) => {
            impl Full<$t> {
                const fn special() -> impl Generate<Item = $t> {
                    struct Special;
                    impl Generate for Special {
                        type Item = $t;
                        type Shrink = $t;

                        fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                            let (item, shrink) = (
                                0 as $t,
                                $t::MIN,
                                $t::MAX,
                                $t::EPSILON,
                                $t::INFINITY,
                                $t::NEG_INFINITY,
                                $t::MIN_POSITIVE,
                                $t::NAN,
                            )
                                .any()
                                .generate(state);
                            (item.fuse(), shrink.fuse())
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

                const fn shrink(item: $t) -> Shrinker<$t> {
                    Shrinker::new(Self::range(), item)
                }
            }

            impl Range<$t> {
                pub fn new(range: impl ops::RangeBounds<$t>) -> Result<Self, Error> {
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

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    let range = self.shrinked(state.size());
                    let ratio = state.random().$t();
                    let difference = range.end * ratio - range.start * ratio;
                    let item = (difference + range.start).max(range.start).min(range.end);
                    (item, Shrinker::new(range, item))
                }
            }

            impl Generate for Full<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
                    fn range(size: f64) -> Range<$t> {
                        Full::<$t>::range().shrinked(size.powi(size_of::<$t>() as i32))
                    }

                    match state.random().u8(..) {
                        0..=126 => range(state.size()).generate(state),
                        127..=253 => range(state.size())
                            .map(|value| 1 as $t / value)
                            .generate(state),
                        254..=255 => {
                            let (item, _) = Full::<$t>::special().generate(state);
                            (item, Full::<$t>::shrink(item))
                        }
                    }
                }
            }

            impl Shrink for Shrinker<$t> {
                type Item = $t;

                fn generate(&self) -> Self::Item {
                    self.item
                }

                fn shrink(&mut self) -> Option<Self> {
                    shrink!(self, $t)
                }
            }

            constant!($t);
            ranges!($t);
        };
        ($($ts:ident),*) => { $(floating!($ts);)* };
    }

    integer!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
    floating!(f32, f64);
}
