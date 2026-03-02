pub mod common;
use common::*;
use std::{
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque},
    rc::Rc,
    sync::Arc,
};

#[test]
fn collections_have_full_generate() {
    assert!(Box::<[u8]>::generator().check(|_| true).is_none());
    assert!(Rc::<[u8]>::generator().check(|_| true).is_none());
    assert!(Arc::<[u8]>::generator().check(|_| true).is_none());
    assert!(Vec::<u8>::generator().check(|_| true).is_none());
    assert!(VecDeque::<u8>::generator().check(|_| true).is_none());
    assert!(LinkedList::<u8>::generator().check(|_| true).is_none());
    assert!(BinaryHeap::<u8>::generator().check(|_| true).is_none());
    assert!(HashSet::<u8>::generator().check(|_| true).is_none());
    assert!(BTreeSet::<u8>::generator().check(|_| true).is_none());
    assert!(HashMap::<u8, u8>::generator().check(|_| true).is_none());
    assert!(BTreeMap::<u8, u8>::generator().check(|_| true).is_none());
}

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check(0usize..=100)]
    fn vec_collect_has_requested_count(count: usize) {
        let vec = (0u8..=u8::MAX).collect_with::<_, Vec<_>>(count).sample(1.0);
        assert_eq!(vec.len(), count);
    }

    #[check(0usize..=100)]
    fn vecdeque_collect_has_requested_count(count: usize) {
        let deque = (0u8..=u8::MAX)
            .collect_with::<_, VecDeque<_>>(count)
            .sample(1.0);
        assert_eq!(deque.len(), count);
    }

    #[check(0usize..=100)]
    fn hashset_collect_has_at_most_requested_count(count: usize) {
        let set = (0u8..=u8::MAX)
            .collect_with::<_, HashSet<_>>(count)
            .sample(1.0);
        assert!(set.len() <= count);
    }
}
