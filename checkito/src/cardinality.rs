use crate::{Generate, primitive::Range, state::State};

#[derive(Debug, Clone)]
pub struct Cardinality<G, const C: u128>(pub(crate) G);

impl<G: Generate, const C: u128> Generate for Cardinality<G, C> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    const CARDINALITY: Option<u128> = Some(C);

    fn generate(&self, state: &mut State) -> Self::Shrink {
        self.0.generate(state)
    }
}

#[inline]
pub(crate) const fn any_sum(left: Option<u128>, right: Option<u128>) -> Option<u128> {
    match (left, right) {
        (Some(left), Some(right)) => u128::checked_add(left, right),
        (None, _) | (_, None) => None,
    }
}

#[inline]
pub(crate) const fn any_repeat_static<const N: usize>(value: Option<u128>) -> Option<u128> {
    match (value, N) {
        (_, 0) => Some(0),
        (Some(value), count) => u128::checked_mul(value, count as _),
        (None, _) => None,
    }
}

#[inline]
pub(crate) const fn all_product(left: Option<u128>, right: Option<u128>) -> Option<u128> {
    match (left, right) {
        (Some(0), _) | (_, Some(0)) => Some(0),
        (Some(left), Some(right)) => u128::checked_mul(left, right),
        (None, _) | (_, None) => None,
    }
}

#[inline]
pub(crate) const fn all_repeat_static<const N: usize>(value: Option<u128>) -> Option<u128> {
    match (value, N) {
        (_, 0) => Some(1),
        (Some(0), _) => Some(0),
        (Some(1), _) => Some(1),
        (Some(value @ 2..), count @ 1..) => {
            if count <= u32::MAX as _ {
                u128::checked_pow(value, count as _)
            } else {
                None
            }
        }
        (None, 1..) => None,
    }
}

pub(crate) const fn all_repeat_dynamic(value: Option<u128>, count: Range<usize>) -> Option<u128> {
    const fn next(value: Option<u128>, count: usize) -> Option<u128> {
        match (value, count) {
            (Some(0), _) | (_, 0) => Some(1),
            (Some(1), count @ 1..) => u128::checked_add(count as _, 1),
            (Some(value @ 2..), count @ 1..) => {
                if count < u32::MAX as _ {
                    match u128::checked_pow(value, count as u32 + 1) {
                        Some(pow) => Some((pow - 1) / (value - 1)),
                        None => None,
                    }
                } else {
                    None
                }
            }
            (None, 1..) => None,
        }
    }

    match next(value, count.end()) {
        Some(end) => match count.start().checked_sub(1) {
            Some(count) => match next(value, count) {
                Some(start) => end.checked_sub(start),
                None => None,
            },
            None => Some(end),
        },
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_repeat_dynamic_is_valid() {
        assert_eq!(all_repeat_dynamic(None, Range::from(0..=1)), None);
        assert_eq!(all_repeat_dynamic(None, Range::from(0..=1000)), None);
        assert_eq!(all_repeat_dynamic(None, Range::from(0..=0)), Some(1));
        assert_eq!(all_repeat_dynamic(Some(1), Range::from(0..=0)), Some(1));
        assert_eq!(all_repeat_dynamic(Some(1), Range::from(0..=10)), Some(11));
        assert_eq!(
            all_repeat_dynamic(Some(1), Range::from(0..=1000)),
            Some(1001)
        );
        assert_eq!(all_repeat_dynamic(Some(2), Range::from(0..=0)), Some(1));
        assert_eq!(all_repeat_dynamic(Some(2), Range::from(0..=1)), Some(1 + 2));
        assert_eq!(
            all_repeat_dynamic(Some(2), Range::from(0..=5)),
            Some(1 + 2 + 4 + 8 + 16 + 32)
        );
        assert_eq!(
            all_repeat_dynamic(Some(3), Range::from(0..=5)),
            Some(1 + 3 + 9 + 27 + 81 + 243)
        );
    }
}
