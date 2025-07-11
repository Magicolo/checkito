use crate::{Generate, state::State};

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
        (Some(value @ 0..=1), _) => Some(value),
        (Some(value), count) => {
            if count <= u32::MAX as _ {
                u128::checked_pow(value, count as _)
            } else {
                None
            }
        }
        (None, _) => None,
    }
}

// pub(crate) const fn all_repeat_dynamic(mut value: Option<u128>, count:
// usize) -> Option<u128> {     // FIXME: This considers only all values
// of [T; count] but not [T; count     // - 1]     // (and so on).
// Example: when T = true, count = 2, the possible     // values are [],
// // [true], [true, true]. This is not represented here.     for i in
// 0..=count {         let a = match (value, count) {
//             (_, 0) => Some(1),
//             (Some(0), _) => Some(0),
//             (Some(1), count @ 1..) => u128::checked_add(count as _, 1),
//             (Some(value @ 2..), count @ 1..) => {
//                 if count <= u32::MAX as _ {
//                     if let Some(result) = u128::checked_pow(value, count
// as _) {                         u128::checked_mul(result, value /
// (value - 1))                     } else {
//                         None
//                     }
//                 } else {
//                     None
//                 }
//             }
//             (None, _) => None,
//         };
//     }
//     value
// }
