use self::usize::Usize;
use crate::{
    any::Any,
    collect::Count,
    generate::{FullGenerate, Generate},
    shrink::Shrink,
    state::{State, Weight},
    utility::{self, tuples},
};
use core::{
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::{self},
};
use orn::{Or2, Or3, Or5};

/// Direction for shrinking numeric values.
///
/// When shrinking a numeric value, we try to move it towards a simpler value:
/// - `None`: The value is already at a boundary and cannot shrink further
/// - `Low`: Shrink by moving towards the lower bound (start of range)
/// - `High`: Shrink by moving towards the upper bound (end of range)
#[derive(Copy, Clone, Debug)]
pub(crate) enum Direction {
    None,
    Low,
    High,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Range<S, E = S>(pub(crate) S, pub(crate) E);

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

pub trait Number: Sized + Copy + Clone + Debug + Display + PartialEq + PartialOrd {
    type Full: Generate<Item = Self>;
    type Positive: Generate<Item = Self>;
    type Negative: Generate<Item = Self>;

    const ZERO: Self;
    const ONE: Self;
    const MIN: Self;
    const MAX: Self;
    const FULL: Self::Full;
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

impl Count for Range<usize> {
    fn count(&self) -> Range<usize> {
        *self
    }
}

impl Count for ops::RangeFrom<usize> {
    fn count(&self) -> Range<usize> {
        self.clone().into()
    }
}

impl Count for ops::Range<usize> {
    fn count(&self) -> Range<usize> {
        self.clone().into()
    }
}

impl Count for ops::RangeInclusive<usize> {
    fn count(&self) -> Range<usize> {
        self.clone().into()
    }
}

impl Count for ops::RangeTo<usize> {
    fn count(&self) -> Range<usize> {
        Range::from(*self)
    }
}

impl Count for ops::RangeToInclusive<usize> {
    fn count(&self) -> Range<usize> {
        Range::from(*self)
    }
}

impl<const N: usize> Count for Usize<N> {
    const COUNT: Option<Range<usize>> = Some(Range(N, N));

    fn count(&self) -> Range<usize> {
        Range(N, N)
    }
}

impl<const N: usize, const M: usize> Count for Range<Usize<N>, Usize<M>> {
    const COUNT: Option<Range<usize>> = Some(Range(N, M));

    fn count(&self) -> Range<usize> {
        Range(N, M)
    }
}

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
                Range(min(N, M), max(N, M))
            }
        }

        impl<const N: $type, const M: $type> Generate for Range<$name<N>, $name<M>> {
            type Item = $type;
            type Shrink = $shrink;

            const CARDINALITY: Option<u128> = cardinality(N, M);

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

            const CARDINALITY: Option<u128> = <Range<$type> as Generate>::CARDINALITY;

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

            const CARDINALITY: Option<u128> = cardinality($type::MIN, $type::MAX);

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Shrinker {
                    start: self.start(),
                    end: self.end(),
                    item: state.$type(self),
                    direction: Direction::None,
                }
            }

            fn cardinality(&self) -> Option<u128> {
                cardinality(self.start(), self.end())
            }
        }
    };
    (FLOATING, $type:ident) => {
        ranges!(RANGES, $type, Shrinker<$type>);

        impl Generate for Range<$type> {
            type Item = $type;
            type Shrink = Shrinker<$type>;

            const CARDINALITY: Option<u128> = utility::$type::cardinality($type::MIN, $type::MAX);

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
                utility::$type::cardinality(self.start(), self.end())
            }
        }
    };
    (CHARACTER, $type:ident) => {
        ranges!(RANGES, $type, Shrinker);

        impl Generate for Range<$type> {
            type Item = $type;
            type Shrink = Shrinker;

            const CARDINALITY: Option<u128> = cardinality($type::MIN, $type::MAX);

            fn generate(&self, state: &mut State) -> Self::Shrink {
                Shrinker(super::Shrinker {
                    start: self.start() as u32,
                    end: self.end() as u32,
                    item: state.$type(self) as u32,
                    direction: Direction::None,
                })
            }

            fn cardinality(&self) -> Option<u128> {
                cardinality(self.start(), self.end())
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

/// Implements `Shrink::shrink` for `Shrinker<$nonzero>`, mirroring the `shrink!` macro but
/// adapted for NonZero types (no 0, shrink toward MIN for unsigned, toward ±1 for signed).
macro_rules! shrink_nonzero {
    (unsigned, $shrink:expr, $inner:ident, $nonzero:ident) => {{
        match $shrink.direction {
            Direction::None => {
                // All unsigned NonZero values are >= 1 = MIN; always shrink toward start.
                if $shrink.start == $shrink.item {
                    None
                } else {
                    $shrink.direction = Direction::High;
                    $shrink.end = $shrink.item;
                    Some(Shrinker {
                        direction: Direction::High,
                        start: $shrink.start,
                        end: $shrink.start,
                        item: $shrink.start,
                    })
                }
            }
            Direction::Low => {
                let start = $shrink.start.get();
                let end = $shrink.end.get();
                let delta = end / 2 as $inner - start / 2 as $inner;
                let middle = start + delta;
                if middle == start || middle == end {
                    None
                } else {
                    // SAFETY: middle is in (start, end) where start >= 1, so middle >= 1.
                    let middle = unsafe { ::core::num::$nonzero::new_unchecked(middle) };
                    let mut shrinker = $shrink.clone();
                    shrinker.start = middle;
                    shrinker.item = middle;
                    $shrink.end = middle;
                    Some(shrinker)
                }
            }
            Direction::High => {
                let start = $shrink.start.get();
                let end = $shrink.end.get();
                let delta = end / 2 as $inner - start / 2 as $inner;
                let middle = start + delta;
                if middle == start || middle == end {
                    None
                } else {
                    // SAFETY: middle is in (start, end) where start >= 1, so middle >= 1.
                    let middle = unsafe { ::core::num::$nonzero::new_unchecked(middle) };
                    let mut shrinker = $shrink.clone();
                    shrinker.end = middle;
                    shrinker.item = middle;
                    $shrink.start = middle;
                    Some(shrinker)
                }
            }
        }
    }};
    (signed, $shrink:expr, $inner:ident, $nonzero:ident) => {{
        match $shrink.direction {
            Direction::None if $shrink.item.get() > 0 => {
                // Positive: shrink toward 1 (the smallest positive NonZero value).
                let one = unsafe { ::core::num::$nonzero::new_unchecked(1 as $inner) };
                $shrink.start = $shrink.start.max(one);
                if $shrink.start == $shrink.item {
                    None
                } else {
                    $shrink.direction = Direction::High;
                    $shrink.end = $shrink.item;
                    Some(Shrinker {
                        direction: Direction::High,
                        start: $shrink.start,
                        end: $shrink.start,
                        item: $shrink.start,
                    })
                }
            }
            Direction::None => {
                // Negative: shrink toward -1 (the largest negative NonZero value).
                let neg_one = unsafe { ::core::num::$nonzero::new_unchecked(-1 as $inner) };
                $shrink.end = $shrink.end.min(neg_one);
                if $shrink.end == $shrink.item {
                    None
                } else {
                    $shrink.direction = Direction::Low;
                    $shrink.start = $shrink.item;
                    Some(Shrinker {
                        direction: Direction::Low,
                        start: $shrink.end,
                        end: $shrink.end,
                        item: $shrink.end,
                    })
                }
            }
            Direction::Low => {
                let start = $shrink.start.get();
                let end = $shrink.end.get();
                let delta = end / 2 as $inner - start / 2 as $inner;
                let middle = start + delta;
                if middle == start || middle == end {
                    None
                } else {
                    // SAFETY: start and end are in the same-sign sub-range (all negative or all
                    // positive), so middle shares that sign and is != 0.
                    let middle = unsafe { ::core::num::$nonzero::new_unchecked(middle) };
                    let mut shrinker = $shrink.clone();
                    shrinker.start = middle;
                    shrinker.item = middle;
                    $shrink.end = middle;
                    Some(shrinker)
                }
            }
            Direction::High => {
                let start = $shrink.start.get();
                let end = $shrink.end.get();
                let delta = end / 2 as $inner - start / 2 as $inner;
                let middle = start + delta;
                if middle == start || middle == end {
                    None
                } else {
                    // SAFETY: start and end are in the same-sign sub-range (all negative or all
                    // positive), so middle shares that sign and is != 0.
                    let middle = unsafe { ::core::num::$nonzero::new_unchecked(middle) };
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

        const CARDINALITY: Option<u128> = cardinality(char::MIN, char::MAX);

        fn generate(&self, state: &mut State) -> Self::Shrink {
            match (
                Weight::one(SPECIAL),
                Weight::new(25.0, Range(Char::MIN, Char::MAX)),
            )
                .generate(state)
            {
                Or2::T0(item) => Shrinker(super::Shrinker {
                    start: 0,
                    end: char::MAX as _,
                    item: item.into::<char>() as _,
                    direction: Direction::None,
                }),
                Or2::T1(shrinker) => shrinker,
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

    pub(crate) const fn min(left: char, right: char) -> char {
        if left < right { left } else { right }
    }

    pub(crate) const fn max(left: char, right: char) -> char {
        if left > right { left } else { right }
    }

    const fn cardinality(start: char, end: char) -> Option<u128> {
        // Subtract surrogate code points (U+D800..=U+DFFF) that fall
        // within [start, end], as they map to REPLACEMENT_CHARACTER.
        let start = start as u32;
        let end = end as u32;
        let surrogates =
            match u32::checked_sub(super::u32::min(end, 0xDFFF), super::u32::max(start, 0xD800)) {
                Some(value) => value + 1,
                None => 0,
            };
        match u128::wrapping_sub(end as _, start as _).checked_add(1) {
            Some(cardinality) => cardinality.checked_sub(surrogates as _),
            None => None,
        }
    }

    full!(char);
    same!(char);
    ranges!(CHARACTER, char);
    constant!(char, Char, Shrinker);
}

macro_rules! nonzero {
    (unsigned, $inner: ident, $constant: ident, $nonzero: ident) => {
        type NZSpecialType = Any<(::core::num::$nonzero, ::core::num::$nonzero)>;
        const NZ_SPECIAL: NZSpecialType =
            Any((::core::num::$nonzero::MIN, ::core::num::$nonzero::MAX));

        impl From<Full<::core::num::$nonzero>> for Range<::core::num::$nonzero> {
            fn from(_: Full<::core::num::$nonzero>) -> Self {
                Range(::core::num::$nonzero::MIN, ::core::num::$nonzero::MAX)
            }
        }

        impl Generate for Special<::core::num::$nonzero> {
            type Item = ::core::num::$nonzero;
            type Shrink = ::core::num::$nonzero;

            const CARDINALITY: Option<u128> = NZSpecialType::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                NZ_SPECIAL.generate(state).into()
            }

            fn cardinality(&self) -> Option<u128> {
                NZ_SPECIAL.cardinality()
            }
        }

        impl Generate for Full<::core::num::$nonzero> {
            type Item = ::core::num::$nonzero;
            type Shrink = Shrinker<::core::num::$nonzero>;

            const CARDINALITY: Option<u128> =
                cardinality(::core::num::$nonzero::MIN.get(), ::core::num::$nonzero::MAX.get());

            fn generate(&self, state: &mut State) -> Self::Shrink {
                match (
                    Weight::one(NZ_SPECIAL),
                    Weight::new(50.0, Range(::core::num::$nonzero::MIN, ::core::num::$nonzero::MAX)),
                )
                .generate(state)
                {
                    Or2::T0(item) => Shrinker {
                        start: ::core::num::$nonzero::MIN,
                        end: ::core::num::$nonzero::MAX,
                        item: item.into(),
                        direction: Direction::None,
                    },
                    Or2::T1(shrinker) => shrinker,
                }
            }
        }

        impl Shrink for Shrinker<::core::num::$nonzero> {
            type Item = ::core::num::$nonzero;

            fn item(&self) -> Self::Item {
                self.item
            }

            fn shrink(&mut self) -> Option<Self> {
                shrink_nonzero!(unsigned, self, $inner, $nonzero)
            }
        }

        impl Generate for Range<::core::num::$nonzero> {
            type Item = ::core::num::$nonzero;
            type Shrink = Shrinker<::core::num::$nonzero>;

            const CARDINALITY: Option<u128> =
                cardinality(::core::num::$nonzero::MIN.get(), ::core::num::$nonzero::MAX.get());

            fn generate(&self, state: &mut State) -> Self::Shrink {
                let start = self.start().get();
                let end = self.end().get();
                // SAFETY: start >= 1 (NonZero guarantees), so the generated item is >= 1.
                let item =
                    unsafe { ::core::num::$nonzero::new_unchecked(state.$inner(Range(start, end))) };
                Shrinker { start: self.start(), end: self.end(), item, direction: Direction::None }
            }

            fn cardinality(&self) -> Option<u128> {
                cardinality(self.start().get(), self.end().get())
            }
        }

        impl Number for ::core::num::$nonzero {
            type Full = Full<::core::num::$nonzero>;
            type Positive = Full<::core::num::$nonzero>;
            type Negative = Full<::core::num::$nonzero>;

            const ZERO: Self = ::core::num::$nonzero::MIN;
            const ONE: Self = ::core::num::$nonzero::MIN;
            const MIN: Self = ::core::num::$nonzero::MIN;
            const MAX: Self = ::core::num::$nonzero::MAX;
            const FULL: Self::Full = Constant::VALUE;
            const POSITIVE: Self::Positive = Constant::VALUE;
            const NEGATIVE: Self::Negative = Constant::VALUE;
        }

        full!(::core::num::$nonzero);
        same!(::core::num::$nonzero);
    };
    (signed, $inner: ident, $constant: ident, $nonzero: ident) => {
        // -1 and 1 are the NonZero values closest to zero; used for shrinking and special values.
        const NZ_NEG_ONE: ::core::num::$nonzero =
            unsafe { ::core::num::$nonzero::new_unchecked(-1 as $inner) };
        const NZ_ONE: ::core::num::$nonzero =
            unsafe { ::core::num::$nonzero::new_unchecked(1 as $inner) };

        type NZSpecialType = Any<(
            ::core::num::$nonzero,
            ::core::num::$nonzero,
            ::core::num::$nonzero,
            ::core::num::$nonzero,
        )>;
        const NZ_SPECIAL: NZSpecialType = Any((
            ::core::num::$nonzero::MIN,
            NZ_NEG_ONE,
            NZ_ONE,
            ::core::num::$nonzero::MAX,
        ));

        impl From<Full<::core::num::$nonzero>> for Range<::core::num::$nonzero> {
            fn from(_: Full<::core::num::$nonzero>) -> Self {
                Range(::core::num::$nonzero::MIN, ::core::num::$nonzero::MAX)
            }
        }

        impl Generate for Special<::core::num::$nonzero> {
            type Item = ::core::num::$nonzero;
            type Shrink = ::core::num::$nonzero;

            const CARDINALITY: Option<u128> = NZSpecialType::CARDINALITY;

            fn generate(&self, state: &mut State) -> Self::Shrink {
                NZ_SPECIAL.generate(state).into()
            }

            fn cardinality(&self) -> Option<u128> {
                NZ_SPECIAL.cardinality()
            }
        }

        impl Generate for Full<::core::num::$nonzero> {
            type Item = ::core::num::$nonzero;
            type Shrink = Shrinker<::core::num::$nonzero>;

            // Full inner type cardinality minus 1 for the excluded zero.
            const CARDINALITY: Option<u128> = match cardinality($inner::MIN, $inner::MAX) {
                Some(c) => c.checked_sub(1),
                None => None,
            };

            fn generate(&self, state: &mut State) -> Self::Shrink {
                // Use two weighted sub-ranges (negative and positive) to avoid generating 0.
                match (
                    Weight::one(NZ_SPECIAL),
                    Weight::new(50.0, Range(::core::num::$nonzero::MIN, NZ_NEG_ONE)),
                    Weight::new(50.0, Range(NZ_ONE, ::core::num::$nonzero::MAX)),
                )
                .generate(state)
                {
                    Or3::T0(item) => Shrinker {
                        start: ::core::num::$nonzero::MIN,
                        end: ::core::num::$nonzero::MAX,
                        item: item.into(),
                        direction: Direction::None,
                    },
                    Or3::T1(shrinker) | Or3::T2(shrinker) => shrinker,
                }
            }
        }

        impl Shrink for Shrinker<::core::num::$nonzero> {
            type Item = ::core::num::$nonzero;

            fn item(&self) -> Self::Item {
                self.item
            }

            fn shrink(&mut self) -> Option<Self> {
                shrink_nonzero!(signed, self, $inner, $nonzero)
            }
        }

        impl Generate for Range<::core::num::$nonzero> {
            type Item = ::core::num::$nonzero;
            type Shrink = Shrinker<::core::num::$nonzero>;

            // Max possible: full non-zero range = inner cardinality minus 1 for excluded zero.
            const CARDINALITY: Option<u128> = match cardinality($inner::MIN, $inner::MAX) {
                Some(c) => c.checked_sub(1),
                None => None,
            };

            fn generate(&self, state: &mut State) -> Self::Shrink {
                let start = self.start().get();
                let end = self.end().get();
                let inner = state.$inner(Range(start, end));
                // SAFETY: NonZero bounds guarantee start and end are != 0, so when the range does
                // not span zero the result is != 0. When it does span zero and 0 is generated we
                // substitute with start (always a valid NonZero value, since start != 0).
                let inner = if inner == 0 as $inner { start } else { inner };
                let item = unsafe { ::core::num::$nonzero::new_unchecked(inner) };
                Shrinker { start: self.start(), end: self.end(), item, direction: Direction::None }
            }

            fn cardinality(&self) -> Option<u128> {
                let start = self.start().get();
                let end = self.end().get();
                let c = cardinality(start, end)?;
                // Subtract 1 when the range spans zero to exclude the missing 0.
                if start < 0 as $inner && end > 0 as $inner { c.checked_sub(1) } else { Some(c) }
            }
        }

        impl Number for ::core::num::$nonzero {
            type Full = Full<::core::num::$nonzero>;
            type Positive = Full<::core::num::$nonzero>;
            type Negative = Full<::core::num::$nonzero>;

            // Zero is represented as 1 (the NonZero value closest to zero).
            const ZERO: Self = NZ_ONE;
            const ONE: Self = NZ_ONE;
            const MIN: Self = ::core::num::$nonzero::MIN;
            const MAX: Self = ::core::num::$nonzero::MAX;
            const FULL: Self::Full = Constant::VALUE;
            const POSITIVE: Self::Positive = Constant::VALUE;
            const NEGATIVE: Self::Negative = Constant::VALUE;
        }

        full!(::core::num::$nonzero);
        same!(::core::num::$nonzero);
    };
}

macro_rules! integer {
    ($type: ident, $constant: ident, $sign: ident, $nonzero: ident) => {
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

            const CARDINALITY: Option<u128> = cardinality($type::MIN, $type::MAX);

            fn generate(&self, state: &mut State) -> Self::Shrink {
                match (
                    Weight::one(SPECIAL),
                    Weight::new(50.0, Range($type::MIN, $type::MAX)))
                .generate(state) {
                    Or2::T0(item) => Shrinker {
                        start: $type::MIN,
                        end: $type::MAX,
                        item: item.into(),
                        direction: Direction::None
                    },
                    Or2::T1(shrinker) => shrinker,
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

            const FULL: Self::Full = Self::Full::VALUE;
            const NEGATIVE: Self::Negative = Self::Negative::VALUE;
            const POSITIVE: Self::Positive = Self::Positive::VALUE;
            const MAX: Self = $type::MAX;
            const MIN: Self = $type::MIN;
            const ONE: Self = 1 as $type;
            const ZERO: Self = 0 as $type;
        }

        pub(crate) const fn min(left: $type, right: $type) -> $type {
            if left < right { left } else { right }
        }

        pub(crate) const fn max(left: $type, right: $type) -> $type {
            if left > right { left } else { right }
        }

        const fn cardinality(start: $type, end: $type) -> Option<u128> {
            u128::wrapping_sub(max(start, end) as _, min(start, end) as _).checked_add(1)
        }

        full!($type);
        same!($type);
        ranges!(INTEGER, $type);
        constant!($type, $constant, Shrinker::<$type>);
        nonzero!($sign, $type, $constant, $nonzero);
    };
    ($([$type: ident, $constant: ident, $sign: ident, $nonzero: ident]),*$(,)?) => { $(pub mod $type { use super::*; integer!($type, $constant, $sign, $nonzero); })* };
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

            // All non-NaN values (finite + both infinities) plus 1 for NaN
            // (all NaN bit patterns are considered the same value).
            const CARDINALITY: Option<u128> = match utility::$type::cardinality($type::NEG_INFINITY, $type::INFINITY) {
                Some(cardinality) => cardinality.checked_add(1),
                None => None,
            };

            fn generate(&self, state: &mut State) -> Self::Shrink {
                match (
                    Weight::one(SPECIAL),
                    Weight::new(10.0, Range($type::MIN, $type::MAX)),
                    Weight::new(10.0, Range(-$type::EPSILON.recip(), $type::EPSILON.recip())),
                    Weight::new(5.0, Range($type::MIN.recip(), $type::MAX.recip())),
                    Weight::new(5.0, Range(-$type::EPSILON, $type::EPSILON)),
                ).generate(state) {
                    Or5::T0(item) => Shrinker {
                        start: $type::MIN,
                        end: $type::MAX,
                        item: item.into(),
                        direction: Direction::None
                    },
                    Or5::T1(shrinker) | Or5::T2(shrinker) | Or5::T3(shrinker) | Or5::T4(shrinker) => shrinker,
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

            const FULL: Self::Full = Range(Self::MIN, Self::MAX);
            const NEGATIVE: Self::Negative = Range(Self::MIN, Self::ZERO);
            const POSITIVE: Self::Positive = Range(Self::ZERO, Self::MAX);
            const MAX: Self = $type::MAX;
            const MIN: Self = $type::MIN;
            const ONE: Self = 1 as $type;
            const ZERO: Self = 0 as $type;
        }

        full!($type);
        same!($type);
        ranges!(FLOATING, $type);
    };
    ($($type: ident),* $(,)?) => { $(pub mod $type { use super::*; floating!($type); })* };
}

macro_rules! tuple {
    ($n:ident, $c:tt $(,$p:ident, $t:ident, $i:tt)*) => {
        impl<$($t: Constant,)*> Constant for ($($t,)*) {
            const VALUE: Self = ($($t::VALUE,)*);
        }
    };
}

integer!(
    [u8, U8, unsigned, NonZeroU8],
    [u16, U16, unsigned, NonZeroU16],
    [u32, U32, unsigned, NonZeroU32],
    [u64, U64, unsigned, NonZeroU64],
    [u128, U128, unsigned, NonZeroU128],
    [usize, Usize, unsigned, NonZeroUsize],
    [i8, I8, signed, NonZeroI8],
    [i16, I16, signed, NonZeroI16],
    [i32, I32, signed, NonZeroI32],
    [i64, I64, signed, NonZeroI64],
    [i128, I128, signed, NonZeroI128],
    [isize, Isize, signed, NonZeroIsize],
);
floating!(f32, f64);

tuples!(tuple);
