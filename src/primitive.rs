use crate::{
    any::Any,
    generate::{FullGenerate, Generate},
    shrink::Shrink,
    state::{Range, State},
    utility,
};
use core::{marker::PhantomData, ops};

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
pub struct Shrinker<T> {
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
        impl FullGenerate for $t {
            type Generator = Full<$t>;
            type Item = $t;

            fn generator() -> Self::Generator {
                Full::<$t>::NEW
            }
        }
    };
}

macro_rules! same {
    ($t:ty) => {
        impl Generate for $t {
            type Item = Self;
            type Shrink = Self;

            const CARDINALITY: Option<usize> = Some(1);

            fn generate(&self, _: &mut State) -> Self::Shrink {
                <$t as Clone>::clone(self)
            }
        }

        impl Shrink for $t {
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
    ($t:ident, $r:ty, $s:ty) => {
        impl Generate for $r {
            type Item = $t;
            type Shrink = $s;

            const CARDINALITY: Option<usize> = $t::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Range::from(self).generate(state)
            }

            fn cardinality(&self) -> Option<usize> {
                Range::from(self).cardinality()
            }
        }
    };
}

macro_rules! ranges {
    (RANGES, $t: ident, $s: ty) => {
        range!($t, ops::Range<$t>, $s);
        range!($t, ops::RangeInclusive<$t>, $s);
        range!($t, ops::RangeFrom<$t>, $s);
        range!($t, ops::RangeTo<$t>, $s);
        range!($t, ops::RangeToInclusive<$t>, $s);
    };
    (INTEGER, $t:ident) => {
        ranges!(RANGES, $t, Shrinker<$t>);

        impl Generate for Range<$t> {
            type Item = $t;
            type Shrink = Shrinker<$t>;

            const CARDINALITY: Option<usize> = $t::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Shrinker {
                    start: self.start(),
                    end: self.end(),
                    item: state.$t(self),
                    direction: Direction::None,
                }
            }

            fn cardinality(&self) -> Option<usize> {
                usize::checked_sub(self.end() as _, self.start() as _)
            }
        }
    };
    (FLOATING, $t:ident) => {
        ranges!(RANGES, $t, Shrinker<$t>);

        impl Generate for Range<$t> {
            type Item = $t;
            type Shrink = Shrinker<$t>;

            const CARDINALITY: Option<usize> = $t::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                debug_assert!(self.start().is_finite() && self.end().is_finite());
                Shrinker {
                    start: self.start(),
                    end: self.end(),
                    item: state.$t(self),
                    direction: Direction::None,
                }
            }

            fn cardinality(&self) -> Option<usize> {
                Some(utility::$t::cardinality(self.start(), self.end()) as _)
            }
        }
    };
    (CHARACTER, $t:ident) => {
        ranges!(RANGES, $t, Shrinker);

        impl Generate for Range<$t> {
            type Item = $t;
            type Shrink = Shrinker;

            const CARDINALITY: Option<usize> = $t::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Shrinker(super::Shrinker {
                    start: self.start() as u32,
                    end: self.end() as u32,
                    item: state.$t(self) as u32,
                    direction: Direction::None,
                })
            }

            fn cardinality(&self) -> Option<usize> {
                usize::checked_sub(self.end() as _, self.start() as _)
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
                    Some(Shrinker {
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
                    Some(Shrinker {
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
    pub struct Shrinker(bool, bool);

    impl Generate for Full<bool> {
        type Item = bool;
        type Shrink = Shrinker;

        const CARDINALITY: Option<usize> = Some(2);

        fn generate(&self, state: &mut State) -> Self::Shrink {
            Shrinker(true, state.bool())
        }
    }

    impl Shrink for Shrinker {
        type Item = bool;

        fn item(&self) -> Self::Item {
            self.1
        }

        fn shrink(&mut self) -> Option<Self> {
            // A distinct `bool` is required to avoid modifying the `item()` if it would be
            // called after shrink.
            if self.1 && take(&mut self.0) {
                Some(Shrinker(false, false))
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
    pub struct Shrinker(super::Shrinker<u32>);

    type SpecialType = Any<(
        char,
        char,
        char,
        char,
        char,
        char,
        char,
        char,
        char,
        char,
        char,
        char,
        char,
        char,
        char,
    )>;

    const SPECIAL: SpecialType = Any((
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
    ));

    impl Generate for Special<char> {
        type Item = char;
        type Shrink = char;

        const CARDINALITY: Option<usize> = SpecialType::CARDINALITY;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            SPECIAL.generate(state).into()
        }

        fn cardinality(&self) -> Option<usize> {
            SPECIAL.cardinality()
        }
    }

    impl Generate for Full<char> {
        type Item = char;
        type Shrink = Shrinker;

        const CARDINALITY: Option<usize> = usize::checked_sub(char::MAX as _, 0 as char as _);

        fn generate(&self, state: &mut State) -> Self::Shrink {
            let value = state.with().size(1.0).u8(..);
            match value {
                0..=249 => Range(0 as char, char::MAX).generate(state),
                250.. => Shrinker(super::Shrinker {
                    start: 0,
                    end: char::MAX as _,
                    item: Special::<char>::NEW.generate(state) as _,
                    direction: Direction::None,
                }),
            }
        }
    }

    impl Shrink for Shrinker {
        type Item = char;

        fn item(&self) -> Self::Item {
            char::from_u32(self.0.item()).unwrap_or(char::REPLACEMENT_CHARACTER)
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
        type Full: Generate<Item = Self>;
        type Special: Generate<Item = Self>;
        type Positive: Generate<Item = Self>;
        type Negative: Generate<Item = Self>;

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
            type SpecialType = Any<($t, $t, $t)>;
            const SPECIAL: SpecialType = Any((0 as $t, $t::MIN, $t::MAX));

            impl Generate for Special<$t> {
                type Item = $t;
                type Shrink = $t;

                const CARDINALITY: Option<usize> = SpecialType::CARDINALITY;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    SPECIAL.generate(state).into()
                }

                fn cardinality(&self) -> Option<usize> {
                    SPECIAL.cardinality()
                }
            }

            impl Generate for Full<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                const CARDINALITY: Option<usize> = usize::checked_sub($t::MAX as _, $t::MIN as _);

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    let value = state.with().size(1.0).u8(..);
                    match value {
                        0..=249 => Range($t::MIN, $t::MAX).generate(state),
                        250.. => Shrinker {
                            start: $t::MIN,
                            end: $t::MAX,
                            item: Special::<$t>::NEW.generate(state),
                            direction: Direction::None
                        },
                    }
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
            same!($t);
            ranges!(INTEGER, $t);
            number!($t);
        };
        ($($ts:ident),*) => { $(pub(crate) mod $ts { use super::*; integer!($ts); })* };
    }

    macro_rules! floating {
        ($t:ident) => {
            type SpecialType = Any<($t, $t, $t, $t, $t, $t, $t, $t)>;
            const SPECIAL: SpecialType = Any((0 as $t, $t::MIN, $t::MAX, $t::EPSILON, $t::INFINITY, $t::NEG_INFINITY, $t::MIN_POSITIVE, $t::NAN));

            impl Generate for Special<$t> {
                type Item = $t;
                type Shrink = $t;

                const CARDINALITY: Option<usize> = SpecialType::CARDINALITY;

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    SPECIAL.generate(state).into()
                }

                fn cardinality(&self) -> Option<usize> {
                    SPECIAL.cardinality()
                }
            }

            impl Generate for Full<$t> {
                type Item = $t;
                type Shrink = Shrinker<$t>;

                const CARDINALITY: Option<usize> = Some(utility::$t::cardinality($t::MIN, $t::MAX) as _);

                fn generate(&self, state: &mut State) -> Self::Shrink {
                    let value = state.with().size(1.0).u8(..);
                    match value {
                        0..=89 => ($t::MIN..=$t::MAX).generate(state),
                        90..=179 => (-$t::EPSILON.recip()..=$t::EPSILON.recip()).generate(state),
                        180..=214 => ($t::MIN.recip()..=$t::MAX.recip()).generate(state),
                        215..=249 => (-$t::EPSILON..=$t::EPSILON).generate(state),
                        250.. => Shrinker {
                            start: $t::MIN,
                            end: $t::MAX,
                            item: Special::<$t>::NEW.generate(state),
                            direction: Direction::None
                        },
                    }
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
