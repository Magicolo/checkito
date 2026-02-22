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
