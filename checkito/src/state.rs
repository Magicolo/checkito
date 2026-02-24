use crate::{
    GENERATES, Generate, Shrink,
    primitive::{Range, u8::U8},
    utility,
};
use core::{
    iter::FusedIterator,
    mem::replace,
    ops::{self, Bound},
};
use fastrand::Rng;
use std::ops::RangeBounds;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sizes {
    range: Range<f64>,
    scale: f64,
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
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
                Some(cardinality) if *index < cardinality => {
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
    pub const fn with(&mut self) -> With<'_> {
        With::new(self)
    }

    #[inline]
    pub const fn descend(&mut self) -> With<'_> {
        let with = self.with();
        with.state.depth += 1;
        with.state.limit += 1;
        with
    }

    #[inline]
    pub const fn dampen(&mut self, deepest: usize, limit: usize, pressure: f64) -> With<'_> {
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

    /// Returns the index of the generator that owns the current exhaustive index,
    /// updating `self.mode` so the index is local to the chosen generator.
    /// Returns `None` in random mode.
    ///
    /// Uses the same cycling interleave logic as `any_exhaustive`: generators are
    /// visited repeatedly in order, proportional to their cardinalities. The loop
    /// terminates for any non-pathological input (at least one non-zero finite
    /// cardinality, or any `None` cardinality). An input where every cardinality
    /// is `Some(0)` would loop indefinitely, matching the analogous behaviour of
    /// `any_exhaustive`.
    pub(crate) fn any_exhaustive_arm(&mut self, cardinalities: &[Option<u128>]) -> Option<usize> {
        let Mode::Exhaustive(index) = &mut self.mode else {
            return None;
        };
        if cardinalities.is_empty() {
            return None;
        }
        loop {
            for (i, &card) in cardinalities.iter().enumerate() {
                match card {
                    Some(c) if *index < c => return Some(i),
                    Some(c) => *index -= c,
                    None => return Some(i),
                }
            }
        }
    }

    pub(crate) fn is_exhaustive(&self) -> bool {
        matches!(self.mode, Mode::Exhaustive(_))
    }

    pub(crate) fn repeat<'a, 'b, G: Generate + ?Sized>(
        &'a mut self,
        generator: &'b G,
        range: Range<usize>,
    ) -> impl Iterator<Item = G::Shrink> + use<'a, 'b, G> {
        let count = match &mut self.mode {
            Mode::Random(_) => range.generate(self).item(),
            Mode::Exhaustive(index) => match generator.cardinality() {
                Some(cardinality) => {
                    // Exhaustive `repeat` chooses a *length* first, then generates
                    // that many items. The selected length must be deterministic for
                    // a given exhaustive `index`.
                    //
                    // For a generator with cardinality `c`:
                    // - length `L` contributes `c^L` distinct combinations.
                    // - so lengths form geometric "buckets" of sizes `c^start, c^(start+1), ...
                    //   c^end`.
                    //
                    // This branch maps the current global exhaustive `index` to the
                    // correct length bucket without scanning linearly.
                    let (start, end) = (range.start(), range.end());
                    // `block` is the size of the first bucket (`c^start`).
                    // We keep it optional: `None` means power overflowed, so we
                    // gracefully fall back to the minimum length.
                    let block = match cardinality {
                        _ if start == 0 => Some(1),
                        0 => Some(0),
                        1 => Some(1),
                        _ => u32::try_from(start)
                            .ok()
                            .and_then(|start| cardinality.checked_pow(start)),
                    };

                    match (cardinality, block) {
                        // Cannot represent `c^start` => conservative fallback.
                        (_, None) => {
                            // Keep the index untouched so sibling generators can
                            // still consume from it.
                            start
                        }
                        // No values can be produced for positive lengths.
                        // Keep behavior conservative and deterministic.
                        (0, Some(_)) => {
                            // No repeat combinations exist in this range. Keep the
                            // index untouched so sibling generators still vary.
                            start
                        }
                        // Each length has exactly one combination, so buckets are
                        // uniform and we can compute the offset directly.
                        (1, Some(_)) => {
                            // `Range<usize>` normalizes bounds so `start <= end`.
                            // Therefore the bucket width is always at least 1.
                            let width = end.saturating_sub(start).saturating_add(1);
                            let total = width as u128;
                            let local = *index % total;
                            *index /= total;
                            let offset = usize::try_from(local).unwrap_or(0);
                            start.saturating_add(offset)
                        }
                        // General case (`c >= 2`): use geometric prefix sums +
                        // binary search to find the selected bucket in O(log width).
                        // Then consume all previous buckets from the index so nested
                        // generation uses an index local to the chosen length.
                        (cardinality, Some(block)) => {
                            let width = end.saturating_sub(start).saturating_add(1);
                            let total = geometric_sum(cardinality, block, width);
                            match total {
                                Some(total @ 1..) => {
                                    let local = *index % total;
                                    let outer = *index / total;
                                    let terms =
                                        select_geometric_terms(local, width, cardinality, block);
                                    let terms_before = terms.saturating_sub(1);
                                    let consumed = geometric_sum(cardinality, block, terms_before)
                                        .unwrap_or(local);
                                    let inner = local.saturating_sub(consumed);
                                    let place = u32::try_from(terms_before)
                                        .ok()
                                        .and_then(|offset| cardinality.checked_pow(offset))
                                        .unwrap_or(1);
                                    *index = outer.saturating_mul(place).saturating_add(inner);
                                    start.saturating_add(terms_before)
                                }
                                Some(0) | None => start,
                            }
                        }
                    }
                }
                // If cardinality is not known, we cannot compute deterministic
                // geometric buckets for lengths. Reuse the repeat-range generator
                // in exhaustive mode, which is deterministic for a given index.
                None => range.generate(self).item(),
            },
        };
        Iterator::map(0..count, move |_| generator.generate(self))
    }
}

const fn consume(index: &mut u128, start: u128, end: u128) -> u128 {
    let range = u128::wrapping_sub(end, start).saturating_add(1);
    let index = replace(index, index.saturating_div(range)) % range;
    u128::wrapping_add(start, index)
}

/// Maps a zero-anchored exhaustive index to a `(offset, is_negative)` pair
/// for generating small-magnitude values first.
///
/// - `pos`: number of steps available in the positive direction.
/// - `neg`: number of steps available in the negative direction.
///
/// Ordering: anchor, +1, −1, +2, −2, … until the shorter side is exhausted,
/// then the remaining steps from the longer side.
const fn small_first(local: u128, pos: u128, neg: u128) -> (u128, bool) {
    let min_side = if pos < neg { pos } else { neg };
    let k = (local + 1) / 2;
    if k <= min_side {
        // Interleaved zone: odd → positive offset, even → negative offset.
        // k == 0 (local == 0) falls here and yields offset 0 (the anchor).
        (k, local % 2 == 0)
    } else {
        // Past the shorter side: continue monotonically on the longer side.
        (local - min_side, pos < neg)
    }
}

/// Computes the number of exhaustive items covered by the first `terms`
/// length-buckets when bucket sizes grow geometrically.
///
/// In plain terms: for repeated generation, each extra length can contribute
/// many more combinations than the previous one (`cardinality` times more).
/// This helper answers “how many total combinations are there up to this
/// length?” using checked arithmetic so overflow is reported as `None`.
fn geometric_sum(cardinality: u128, block: u128, terms: usize) -> Option<u128> {
    if terms == 0 {
        return Some(0);
    }
    if cardinality == 1 {
        return u128::try_from(terms).ok();
    }
    let terms = u32::try_from(terms).ok()?;
    let factor = cardinality.checked_pow(terms)?.checked_sub(1)?;
    block.checked_mul(factor)?.checked_div(cardinality - 1)
}

/// Finds how many geometric buckets are needed so their cumulative size is
/// strictly greater than `index`.
///
/// This lets exhaustive `repeat` choose the output length in `O(log width)`
/// instead of scanning every possible length in `start..=end`.
///
/// If cumulative sums overflow, we conservatively treat that as “large enough”,
/// which keeps the search deterministic and avoids linear work.
fn select_geometric_terms(index: u128, width: usize, cardinality: u128, block: u128) -> usize {
    if width <= 1 {
        return 1;
    }

    let mut low = 1usize;
    let mut high = width;
    while low < high {
        let mid = low + (high - low) / 2;
        let enough = match geometric_sum(cardinality, block, mid) {
            Some(sum) => index < sum,
            None => true,
        };
        if enough {
            high = mid;
        } else {
            low = mid.saturating_add(1);
        }
    }
    low
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
                                start.wrapping_add(value)
                            } else if end <= 0 {
                                debug_assert!(start < 0);
                                end.wrapping_sub(value)
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
                        Mode::Exhaustive(index) => {
                            // Generate small values (near zero) first by reordering the
                            // exhaustive index instead of enumerating linearly from `start`.
                            let range = (start as u128, end as u128);
                            let total = u128::wrapping_sub(range.1, range.0);
                            let local = consume(index, 0, total);
                            #[allow(unused_comparisons)]
                            if start >= 0 {
                                // Entirely non-negative: ascending from start.
                                u128::wrapping_add(range.0, local) as $integer
                            } else if end <= 0 {
                                // Entirely non-positive: descending from end (closest to zero).
                                u128::wrapping_sub(range.1, local) as $integer
                            } else {
                                // Spans zero: 0, 1, -1, 2, -2, … via small_first.
                                let (value, negative) = small_first(local, end as u128, 0u128.wrapping_sub(range.0));
                                if negative {
                                    0u128.wrapping_sub(value) as $integer
                                } else {
                                    value as $integer
                                }
                            }
                        }
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
                        Mode::Exhaustive(index) => {
                            // Generate small-magnitude values first by working in the
                            // total-order bit space.  The transformation mirrors the
                            // integer strategy: non-positive ranges start from the
                            // end closest to zero; ranges spanning zero interleave
                            // positive and negative values by magnitude.
                            let range = (
                                utility::$number::to_bits(start) as u128,
                                utility::$number::to_bits(end) as u128
                            );
                            let total = range.1 - range.0;
                            let local = consume(index, 0, total);
                            if start >= 0.0 {
                                // Entirely non-negative: go upward from start.
                                utility::$number::from_bits((range.0 + local) as _)
                            } else if end <= 0.0 {
                                // Entirely non-positive: go downward from end (closest to 0).
                                utility::$number::from_bits((range.1 - local) as _)
                            } else {
                                // Spans zero: 0.0, +ε, -ε, +2ε, -2ε, … via small_first.
                                let zero = utility::$number::to_bits(0.0) as u128;
                                let (value, negative) = small_first(local, range.1 - zero, zero - range.0);
                                if negative {
                                    utility::$number::from_bits((zero - value) as _)
                                } else {
                                    utility::$number::from_bits((zero + value) as _)
                                }
                            }
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

    pub(crate) fn state(self, index: usize) -> Option<State> {
        match self {
            Modes::Random { count, sizes, seed } if index < count => {
                Some(State::random(index, count, sizes, seed))
            }
            Modes::Exhaustive(count) if index < count => Some(State::exhaustive(index, count)),
            Modes::Random { .. } | Modes::Exhaustive(..) => None,
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
        self.modes.state(self.indices.next()?)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }

    fn count(self) -> usize {
        self.indices.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.modes.state(self.indices.nth(n)?)
    }

    fn last(self) -> Option<Self::Item> {
        self.modes.state(self.indices.last()?)
    }
}

impl ExactSizeIterator for States {
    fn len(&self) -> usize {
        self.indices.len()
    }
}

impl DoubleEndedIterator for States {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.modes.state(self.indices.next_back()?)
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.modes.state(self.indices.nth_back(n)?)
    }
}

impl FusedIterator for States {}

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

    #[test]
    fn exhaustive_integer_small_values_first() {
        fn collect(count: usize, generate: impl Fn(&mut State) -> i32) -> Vec<i32> {
            Iterator::map(0..count, |i| generate(&mut State::exhaustive(i, count))).collect()
        }

        // Non-negative range: ascending from start.
        let values = collect(5, |s| s.i32(0..=4));
        assert_eq!(values, [0, 1, 2, 3, 4]);

        // Non-positive range: descending from end (end is closest to 0).
        let values = collect(5, |s| s.i32(-4..=0));
        assert_eq!(values, [0, -1, -2, -3, -4]);

        // Symmetric range spanning 0: interleaved 0, 1, -1, 2, -2, ...
        let values = collect(5, |s| s.i32(-2..=2));
        assert_eq!(values, [0, 1, -1, 2, -2]);

        // Asymmetric range with more positives: interleave up to the smaller side,
        // then continue with the remaining positives.
        let values = collect(7, |s| s.i32(-2..=4));
        assert_eq!(values, [0, 1, -1, 2, -2, 3, 4]);

        // Asymmetric range with more negatives: interleave up to the smaller side,
        // then continue with the remaining negatives.
        let values = collect(7, |s| s.i32(-4..=2));
        assert_eq!(values, [0, 1, -1, 2, -2, -3, -4]);

        // First few exhaustive values for a large range should be near zero.
        let values = collect(5, |s| s.i32(-1000..=1000));
        assert_eq!(values, [0, 1, -1, 2, -2]);
    }

    #[test]
    fn exhaustive_float_small_values_first() {
        // Non-positive range: first value must be 0.0 (the end, closest to 0).
        let first = State::exhaustive(0, 100).f32(-1.0..=0.0);
        assert_eq!(first, 0.0);

        // Range spanning 0: first value is 0.0.
        let first = State::exhaustive(0, 1000).f32(-1.0..=1.0);
        assert_eq!(first, 0.0);

        // For a range spanning 0, the total-order magnitude should increase for
        // the first few samples.  (Use to_bits for total-order comparisons so
        // that -0.0 and 0.0 are treated as distinct adjacent values.)
        let zero_bits = utility::f32::to_bits(0.0);
        let values: Vec<u32> = Iterator::map(0..5, |i| {
            utility::f32::to_bits(State::exhaustive(i, 1000).f32(-1.0..=1.0))
        })
        .collect();
        // index 0 → 0.0, index 1 → small positive (> 0.0 in total order),
        // index 2 → small negative (< 0.0 in total order but closest to it),
        // index 3 → next positive (larger than index 1), index 4 → next negative.
        assert_eq!(values[0], zero_bits);
        assert!(values[1] > zero_bits && values[1] < values[3]);
        assert!(values[2] < zero_bits && values[2] > values[4]);
    }
}
