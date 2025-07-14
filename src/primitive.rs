use crate::{
    any::Any,
    collect::Count,
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

pub trait Constant {
    const VALUE: Self;
}

impl<T> Constant for Full<T> {
    const VALUE: Self = Self(PhantomData);
}

impl<T> Constant for Special<T> {
    const VALUE: Self = Self(PhantomData);
}

impl<S: Constant, E: Constant> Constant for Range<S, E> {
    const VALUE: Self = Self(S::VALUE, E::VALUE);
}

impl<T: ?Sized> Clone for Special<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for Special<T> {}

impl<T: ?Sized> Clone for Full<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for Full<T> {}

impl Count for usize {
    fn count(&self) -> Range<usize> {
        Range::from(*self)
    }
}

impl Count for Full<usize> {
    const COUNT: Option<Range<usize>> = Some(Range(usize::MIN, usize::MAX));

    fn count(&self) -> Range<usize> {
        Range(usize::MIN, usize::MAX)
    }
}

macro_rules! full {
    ($type: ty) => {
        impl FullGenerate for $type {
            type Generator = Full<$type>;
            type Item = $type;

            fn generator() -> Self::Generator {
                Constant::VALUE
            }
        }
    };
}

macro_rules! same {
    ($type: ty) => {
        impl Generate for $type {
            type Item = Self;
            type Shrink = Self;

            const CARDINALITY: Option<u128> = Some(1);

            fn generate(&self, _: &mut State) -> Self::Shrink {
                <$type as Clone>::clone(self)
            }
        }

        impl Shrink for $type {
            type Item = Self;

            fn item(&self) -> Self::Item {
                <$type as Clone>::clone(self)
            }

            fn shrink(&mut self) -> Option<Self> {
                None
            }
        }
    };
}

macro_rules! constant {
    ($type: ident, $name: ident, $shrink: ty) => {
        #[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name<const N: $type>;

        impl $name<{ $type::MIN }> {
            pub const MIN: Self = Self;
        }

        impl $name<{ $type::MAX }> {
            pub const MAX: Self = Self;
        }

        impl $name<{ 0 as $type }> {
            pub const ZERO: Self = Self;
        }

        impl $name<{ 1 as $type }> {
            pub const ONE: Self = Self;
        }

        impl<const N: $type> From<$name<N>> for $type {
            fn from(_: $name<N>) -> Self {
                N
            }
        }

        impl<const N: $type> Constant for $name<N> {
            const VALUE: Self = Self;
        }

        impl<const N: $type> Generate for $name<N> {
            type Item = $type;
            type Shrink = $type;

            const CARDINALITY: Option<u128> = Some(1);

            fn generate(&self, _: &mut State) -> Self::Shrink {
                N
            }
        }

        impl<const N: $type> From<$name<N>> for Range<$name<N>, $name<N>> {
            fn from(value: $name<N>) -> Self {
                Range(value, value)
            }
        }

        impl<const N: $type, const M: $type> From<Range<$name<N>, $name<M>>> for Range<$type> {
            fn from(_: Range<$name<N>, $name<M>>) -> Self {
                Range(
                    if N < M { N } else { M } as _,
                    if N < M { M } else { N } as _,
                )
            }
        }

        impl<const N: $type, const M: $type> Generate for Range<$name<N>, $name<M>> {
            type Item = $type;
            type Shrink = $shrink;

            const CARDINALITY: Option<u128> = Some(u128::wrapping_sub(
                if N < M { M } else { N } as _,
                if N < M { N } else { M } as _,
            ));

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Range::<$type>::from(*self).generate(state)
            }
        }
    };
}

macro_rules! range {
    ($type: ident, $range: ty, $shrink: ty) => {
        impl Generate for $range {
            type Item = $type;
            type Shrink = $shrink;

            const CARDINALITY: Option<u128> = $type::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Range::from(self).generate(state)
            }

            fn cardinality(&self) -> Option<u128> {
                Range::from(self).cardinality()
            }
        }
    };
}

macro_rules! ranges {
    (RANGES, $type: ident, $shrink: ty) => {
        range!($type, ops::Range<$type>, $shrink);
        range!($type, ops::RangeInclusive<$type>, $shrink);
        range!($type, ops::RangeFrom<$type>, $shrink);
        range!($type, ops::RangeTo<$type>, $shrink);
        range!($type, ops::RangeToInclusive<$type>, $shrink);
    };
    (INTEGER, $type: ident) => {
        ranges!(RANGES, $type, Shrinker<$type>);

        impl Generate for Range<$type> {
            type Item = $type;
            type Shrink = Shrinker<$type>;

            const CARDINALITY: Option<u128> = $type::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Shrinker {
                    start: self.start(),
                    end: self.end(),
                    item: state.$type(self),
                    direction: Direction::None,
                }
            }

            fn cardinality(&self) -> Option<u128> {
                Some(u128::wrapping_sub(self.end() as _, self.start() as _))
            }
        }
    };
    (FLOATING, $type:ident) => {
        ranges!(RANGES, $type, Shrinker<$type>);

        impl Generate for Range<$type> {
            type Item = $type;
            type Shrink = Shrinker<$type>;

            const CARDINALITY: Option<u128> = $type::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                debug_assert!(self.start().is_finite() && self.end().is_finite());
                Shrinker {
                    start: self.start(),
                    end: self.end(),
                    item: state.$type(self),
                    direction: Direction::None,
                }
            }

            fn cardinality(&self) -> Option<u128> {
                Some(utility::$type::cardinality(self.start(), self.end()) as _)
            }
        }
    };
    (CHARACTER, $type:ident) => {
        ranges!(RANGES, $type, Shrinker);

        impl Generate for Range<$type> {
            type Item = $type;
            type Shrink = Shrinker;

            const CARDINALITY: Option<u128> = $type::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Shrinker(super::Shrinker {
                    start: self.start() as u32,
                    end: self.end() as u32,
                    item: state.$type(self) as u32,
                    direction: Direction::None,
                })
            }

            fn cardinality(&self) -> Option<u128> {
                Some(u128::wrapping_sub(self.end() as _, self.start() as _))
            }
        }
    };
}

macro_rules! shrink {
    ($shrink:expr, $type:ident) => {{
        // Never change `$shrink.item` to preserve coherence in calls to
        // `shrinker.item()`.
        match $shrink.direction {
            Direction::None if $shrink.item >= 0 as $type => {
                $shrink.start = $shrink.start.max(0 as $type);
                if $shrink.start == $shrink.item {
                    None
                } else {
                    $shrink.direction = Direction::High;
                    $shrink.end = $shrink.item;
                    Some(Shrinker {
                        direction: $shrink.direction,
                        start: $shrink.start,
                        end: $shrink.start,
                        item: $shrink.start,
                    })
                }
            }
            Direction::None => {
                $shrink.end = $shrink.end.min(0 as $type);
                if $shrink.end == $shrink.item {
                    None
                } else {
                    $shrink.direction = Direction::Low;
                    $shrink.start = $shrink.item;
                    Some(Shrinker {
                        direction: $shrink.direction,
                        start: $shrink.end,
                        end: $shrink.end,
                        item: $shrink.end,
                    })
                }
            }
            Direction::Low => {
                let delta = $shrink.end / 2 as $type - $shrink.start / 2 as $type;
                let middle = $shrink.start + delta;
                if middle == $shrink.start || middle == $shrink.end {
                    None
                } else {
                    let mut shrinker = $shrink.clone();
                    shrinker.start = middle;
                    shrinker.item = middle;
                    $shrink.end = middle;
                    Some(shrinker)
                }
            }
            Direction::High => {
                let delta = $shrink.end / 2 as $type - $shrink.start / 2 as $type;
                let middle = $shrink.start + delta;
                if middle == $shrink.start || middle == $shrink.end {
                    None
                } else {
                    let mut shrinker = $shrink.clone();
                    shrinker.end = middle;
                    shrinker.item = middle;
                    $shrink.start = middle;
                    Some(shrinker)
                }
            }
        }
    }};
}

same!(&str);
same!(Box<str>);
same!(String);

pub mod bool {
    use super::*;
    use core::mem::take;

    #[derive(Debug, Copy, Clone, Default)]
    pub struct Bool<const N: bool>;
    #[derive(Copy, Clone, Debug)]
    pub struct Shrinker(bool, bool);

    impl<const N: bool> From<Bool<N>> for bool {
        fn from(_: Bool<N>) -> Self {
            N
        }
    }

    impl Bool<true> {
        pub const TRUE: Self = Self;
    }

    impl Bool<false> {
        pub const FALSE: Self = Self;
    }

    impl<const N: bool> Constant for Bool<N> {
        const VALUE: Self = Self;
    }

    impl<const N: bool> Generate for Bool<N> {
        type Item = bool;
        type Shrink = bool;

        const CARDINALITY: Option<u128> = Some(1);

        fn generate(&self, _: &mut State) -> Self::Shrink {
            N
        }
    }

    impl Generate for Full<bool> {
        type Item = bool;
        type Shrink = Shrinker;

        const CARDINALITY: Option<u128> = Some(2);

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

        const CARDINALITY: Option<u128> = SpecialType::CARDINALITY;

        fn generate(&self, state: &mut State) -> Self::Shrink {
            SPECIAL.generate(state).into()
        }

        fn cardinality(&self) -> Option<u128> {
            SPECIAL.cardinality()
        }
    }

    impl Generate for Full<char> {
        type Item = char;
        type Shrink = Shrinker;

        const CARDINALITY: Option<u128> = Some(u128::wrapping_sub(char::MAX as _, 0 as char as _));

        fn generate(&self, state: &mut State) -> Self::Shrink {
            let value = state.with().size(1.0).u8(..);
            match value {
                0..=249 => Range(Char::MIN, Char::MAX).generate(state),
                250.. => Shrinker(super::Shrinker {
                    start: 0,
                    end: char::MAX as _,
                    item: Special::<char>::VALUE.generate(state) as _,
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
    constant!(char, Char, Shrinker);
}

macro_rules! integer {
    ($type: ident, $constant: ident) => {
        type SpecialType = Any<($type, $type, $type)>;
        const SPECIAL: SpecialType = Any((0 as $type, $type::MIN, $type::MAX));

        impl From<Full<$type>> for Range<$type> {
            fn from(_: Full<$type>) -> Self {
                Range($type::MIN, $type::MAX)
            }
        }

        impl Generate for Special<$type> {
            type Item = $type;
            type Shrink = $type;

            const CARDINALITY: Option<u128> = SpecialType::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                SPECIAL.generate(state).into()
            }

            fn cardinality(&self) -> Option<u128> {
                SPECIAL.cardinality()
            }
        }

        impl Generate for Full<$type> {
            type Item = $type;
            type Shrink = Shrinker<$type>;

            const CARDINALITY: Option<u128> = Some(u128::wrapping_sub($type::MAX as _, $type::MIN as _));

            fn generate(&self, state: &mut State) -> Self::Shrink {
                let value = state.with().size(1.0).u8(..);
                match value {
                    0..=249 => Range($type::MIN, $type::MAX).generate(state),
                    250.. => Shrinker {
                        start: $type::MIN,
                        end: $type::MAX,
                        item: Special::<$type>::VALUE.generate(state),
                        direction: Direction::None
                    },
                }
            }
        }

        impl Shrink for Shrinker<$type> {
            type Item = $type;

            fn item(&self) -> Self::Item {
                self.item
            }

            fn shrink(&mut self) -> Option<Self> {
                shrink!(self, $type)
            }
        }

        impl Number for $type {
            type Full = Range<$constant::<{ Self::MIN }>, $constant::<{ Self::MAX }>>;
            type Negative = Range<$constant::<{ Self::MIN }>, $constant::<{ Self::ZERO }>>;
            type Positive = Range<$constant::<{ Self::ZERO }>, $constant::<{ Self::MAX }>>;
            type Special = Special<Self>;

            const FULL: Self::Full = Self::Full::VALUE;
            const NEGATIVE: Self::Negative = Self::Negative::VALUE;
            const POSITIVE: Self::Positive = Self::Positive::VALUE;
            const SPECIAL: Self::Special = Self::Special::VALUE;
            const MAX: Self = $type::MAX;
            const MIN: Self = $type::MIN;
            const ONE: Self = 1 as $type;
            const ZERO: Self = 0 as $type;
        }

        full!($type);
        same!($type);
        ranges!(INTEGER, $type);
        constant!($type, $constant, Shrinker::<$type>);
    };
    ($([$type: ident, $constant: ident]),*$(,)?) => { $(pub mod $type { use super::*; integer!($type, $constant); })* };
}

macro_rules! floating {
    ($type: ident) => {
        type SpecialType = Any<($type, $type, $type, $type, $type, $type, $type, $type)>;
        const SPECIAL: SpecialType = Any((0 as $type, $type::MIN, $type::MAX, $type::EPSILON, $type::INFINITY, $type::NEG_INFINITY, $type::MIN_POSITIVE, $type::NAN));

        impl Generate for Special<$type> {
            type Item = $type;
            type Shrink = $type;

            const CARDINALITY: Option<u128> = SpecialType::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                SPECIAL.generate(state).into()
            }

            fn cardinality(&self) -> Option<u128> {
                SPECIAL.cardinality()
            }
        }

        impl Generate for Full<$type> {
            type Item = $type;
            type Shrink = Shrinker<$type>;

            const CARDINALITY: Option<u128> = Some(utility::$type::cardinality($type::MIN, $type::MAX) as _);

            fn generate(&self, state: &mut State) -> Self::Shrink {
                let value = state.with().size(1.0).u8(..);
                match value {
                    0..=89 => ($type::MIN..=$type::MAX).generate(state),
                    90..=179 => (-$type::EPSILON.recip()..=$type::EPSILON.recip()).generate(state),
                    180..=214 => ($type::MIN.recip()..=$type::MAX.recip()).generate(state),
                    215..=249 => (-$type::EPSILON..=$type::EPSILON).generate(state),
                    250.. => Shrinker {
                        start: $type::MIN,
                        end: $type::MAX,
                        item: Special::<$type>::VALUE.generate(state),
                        direction: Direction::None
                    },
                }
            }
        }

        impl Shrink for Shrinker<$type> {
            type Item = $type;

            fn item(&self) -> Self::Item {
                self.item
            }

            fn shrink(&mut self) -> Option<Self> {
                if self.item.is_finite() {
                    shrink!(self, $type)
                } else {
                    None
                }
            }
        }


        impl Number for $type {
            type Full = Range<$type>;
            type Negative = Range<$type>;
            type Positive = Range<$type>;
            type Special = Special<Self>;

            const FULL: Self::Full = Range(Self::MIN, Self::MAX);
            const NEGATIVE: Self::Negative = Range(Self::MIN, Self::ZERO);
            const POSITIVE: Self::Positive = Range(Self::ZERO, Self::MAX);
            const SPECIAL: Self::Special = Self::Special::VALUE;
            const MAX: Self = $type::MAX;
            const MIN: Self = $type::MIN;
            const ONE: Self = 1 as $type;
            const ZERO: Self = 0 as $type;
        }

        full!($type);
        same!($type);
        ranges!(FLOATING, $type);
    };
    ($($types: ident),*) => { $(pub mod $types { use super::*; floating!($types); })* };
}

integer!(
    [u8, U8],
    [u16, U16],
    [u32, U32],
    [u64, U64],
    [u128, U128],
    [usize, Usize],
    [i8, I8],
    [i16, I16],
    [i32, I32],
    [i64, I64],
    [i128, I128],
    [isize, Isize],
);
floating!(f32, f64);
