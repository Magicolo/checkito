pub mod common;
use common::*;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};

#[test]
fn vec_deque_generates() {
    assert!(VecDeque::<u8>::generator().check(|_| true).is_none());
}

#[test]
fn linked_list_generates() {
    assert!(LinkedList::<u8>::generator().check(|_| true).is_none());
}

#[test]
fn binary_heap_generates() {
    assert!(BinaryHeap::<u8>::generator().check(|_| true).is_none());
}

#[test]
fn hash_set_generates() {
    assert!(HashSet::<u8>::generator().check(|_| true).is_none());
}

#[test]
fn btree_set_generates() {
    assert!(BTreeSet::<u8>::generator().check(|_| true).is_none());
}

#[test]
fn hash_map_generates() {
    assert!(HashMap::<u8, u8>::generator().check(|_| true).is_none());
}

#[test]
fn btree_map_generates() {
    assert!(BTreeMap::<u8, u8>::generator().check(|_| true).is_none());
}

#[test]
fn hash_map_entries_are_key_value_pairs() {
    let fail = HashMap::<u8, u8>::generator()
        .check(|map| map.iter().all(|(k, v)| k == v))
        .unwrap();
    // At least two distinct elements found so k != v
    assert!(fail.item.iter().any(|(k, v)| k != v));
}

#[test]
fn btree_map_is_ordered() {
    assert!(BTreeMap::<u8, u8>::generator()
        .check(|map| {
            let keys: Vec<_> = map.keys().collect();
            keys.windows(2).all(|w| w[0] <= w[1])
        })
        .is_none());
}

#[test]
fn btree_set_is_ordered() {
    assert!(BTreeSet::<u8>::generator()
        .check(|set| {
            let items: Vec<_> = set.iter().collect();
            items.windows(2).all(|w| w[0] <= w[1])
        })
        .is_none());
}

#[test]
fn binary_heap_contains_ordered_elements() {
    assert!(BinaryHeap::<u8>::generator()
        .check(|heap| {
            let items = heap.into_sorted_vec();
            items.windows(2).all(|w| w[0] <= w[1])
        })
        .is_none());
}
