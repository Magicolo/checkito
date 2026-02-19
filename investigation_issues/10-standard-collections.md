# Add Support for Standard Collection Types (HashMap, BTreeMap, HashSet, etc.)

## Summary
The `FullGenerate` trait currently only has implementations for `Vec<T>` and `String`, but not for other standard collection types like `HashMap`, `BTreeMap`, `HashSet`, `BTreeSet`, `VecDeque`, `LinkedList`, and `BinaryHeap`.

## Context
Users frequently need to test properties involving standard collections. Currently, they must manually implement `Generate` for these types or use workarounds with `Vec` and `collect()`.

## Current State

**Location**: `checkito/src/collect.rs:168-183`

**What's Implemented**:
```rust
// Lines 168-174: Vec implementation
impl<G: Generate> FullGenerate for Vec<G> {
    type Generate = Collect<G::Generate>;
    fn generator() -> Self::Generate {
        Collect::new(G::generator())
    }
}

// Lines 177-183: String implementation  
impl FullGenerate for String {
    type Generate = Collect<char::Generate>;
    fn generator() -> Self::Generate {
        Collect::new(char::generator())
    }
}
```

**What's Missing**:
- `HashMap<K, V>`
- `BTreeMap<K, V>`
- `HashSet<T>`
- `BTreeSet<T>`
- `VecDeque<T>`
- `LinkedList<T>`
- `BinaryHeap<T>` (requires `Ord`)

## Requested Implementations

### HashMap<K, V>
```rust
impl<K, V> FullGenerate for HashMap<K, V>
where
    K: FullGenerate + Eq + Hash,
    V: FullGenerate,
{
    type Generate = Collect<(K::Generate, V::Generate), HashMap<K, V>>;
    
    fn generator() -> Self::Generate {
        Collect::new((K::generator(), V::generator()))
            .map(|pairs| pairs.into_iter().collect())
    }
}
```

**Use Case**:
```rust
#[check(_)]  // Now works!
fn test_hashmap(map: HashMap<String, i32>) {
    for (key, value) in &map {
        assert!(key.len() > 0);
        assert!(value >= 0);
    }
}
```

### BTreeMap<K, V>
```rust
impl<K, V> FullGenerate for BTreeMap<K, V>
where
    K: FullGenerate + Ord,
    V: FullGenerate,
{
    type Generate = Collect<(K::Generate, V::Generate), BTreeMap<K, V>>;
    
    fn generator() -> Self::Generate {
        Collect::new((K::generator(), V::generator()))
            .map(|pairs| pairs.into_iter().collect())
    }
}
```

### HashSet<T>
```rust
impl<T> FullGenerate for HashSet<T>
where
    T: FullGenerate + Eq + Hash,
{
    type Generate = Collect<T::Generate, HashSet<T>>;
    
    fn generator() -> Self::Generate {
        Collect::new(T::generator())
            .map(|vec| vec.into_iter().collect())
    }
}
```

### BTreeSet<T>
```rust
impl<T> FullGenerate for BTreeSet<T>
where
    T: FullGenerate + Ord,
{
    type Generate = Collect<T::Generate, BTreeSet<T>>;
    
    fn generator() -> Self::Generate {
        Collect::new(T::generator())
            .map(|vec| vec.into_iter().collect())
    }
}
```

### VecDeque<T>
```rust
impl<T> FullGenerate for VecDeque<T>
where
    T: FullGenerate,
{
    type Generate = Collect<T::Generate, VecDeque<T>>;
    
    fn generator() -> Self::Generate {
        Collect::new(T::generator())
            .map(|vec| vec.into_iter().collect())
    }
}
```

### LinkedList<T>
```rust
impl<T> FullGenerate for LinkedList<T>
where
    T: FullGenerate,
{
    type Generate = Collect<T::Generate, LinkedList<T>>;
    
    fn generator() -> Self::Generate {
        Collect::new(T::generator())
            .map(|vec| vec.into_iter().collect())
    }
}
```

### BinaryHeap<T>
```rust
impl<T> FullGenerate for BinaryHeap<T>
where
    T: FullGenerate + Ord,
{
    type Generate = Collect<T::Generate, BinaryHeap<T>>;
    
    fn generator() -> Self::Generate {
        Collect::new(T::generator())
            .map(|vec| vec.into_iter().collect())
    }
}
```

## Shrinking Behavior

### Sets and Maps
Special considerations:
1. **Uniqueness**: Sets and map keys must be unique
   - Shrinking may produce duplicates that get deduplicated
   - Cardinality changes during shrinking (5 elements → 3 after dedup)
   
2. **Ordering**: BTree collections are ordered
   - Shrinking should preserve order properties where relevant

**Recommendation**: 
- Shrink by reducing collection size first
- Then shrink individual elements
- Handle duplicates by filtering during shrinking

### Example Shrinking for HashSet
```rust
impl Shrink for HashSet<T>
where
    T: Shrink + Eq + Hash,
{
    type Shrink = CollectShrink<T::Shrink, HashSet<T>>;
    
    fn shrink(&self) -> Self::Shrink {
        // Convert to Vec, shrink, then dedup
        let vec: Vec<_> = self.iter().cloned().collect();
        CollectShrink::from_vec(vec)
            .map(|v| v.into_iter().collect())
    }
}
```

## Benefits

1. **Ergonomics**: Users can use `_` inference for all standard collections
2. **Consistency**: All std collections work the same way
3. **Completeness**: Covers all major collection types
4. **Type Safety**: Compiler ensures proper trait bounds (Hash, Ord, etc.)

## Testing Strategy

Create `tests/collections.rs`:
```rust
#[check(_)]
fn hashmap_keys_are_valid(map: HashMap<String, i32>) {
    for key in map.keys() {
        assert!(key.len() < 100);
    }
}

#[check(_)]
fn btreemap_is_sorted(map: BTreeMap<i32, String>) {
    let keys: Vec<_> = map.keys().collect();
    assert!(keys.windows(2).all(|w| w[0] <= w[1]));
}

#[check(_)]
fn hashset_no_duplicates(set: HashSet<i32>) {
    // By definition, sets have no duplicates
    // Test shrinking preserves uniqueness
    assert_eq!(set.len(), set.iter().collect::<HashSet<_>>().len());
}

#[check(_)]
fn vecdeque_behaves_like_vec(deque: VecDeque<char>) {
    let vec: Vec<_> = deque.iter().cloned().collect();
    assert_eq!(deque.len(), vec.len());
}
```

## Implementation Notes

### File Location
Add implementations to `checkito/src/standard.rs` or `checkito/src/collect.rs`

### Generic Over FromIterator
Consider a generic implementation:
```rust
impl<T, C> FullGenerate for C
where
    C: FromIterator<T>,
    T: FullGenerate,
{
    type Generate = Collect<T::Generate, C>;
    fn generator() -> Self::Generate {
        Collect::new(T::generator())
            .map(|vec| vec.into_iter().collect())
    }
}
```

**Problem**: Too general, conflicts with existing impls
**Solution**: Explicit impls for each collection type

### Cardinality for Maps/Sets
```rust
// For HashMap<K, V>: cardinality is combination of key and value cardinalities
// But uniqueness constraint makes exact calculation complex
// Conservative estimate: return None (unknown)
```

## Priority
**Medium-High** - Common use case, frequently requested feature

## Related Code
- `checkito/src/collect.rs`: Lines 168-183 (existing Vec/String impls)
- `checkito/src/generate.rs`: FullGenerate trait definition

## Acceptance Criteria
- [ ] All standard collection types have FullGenerate impls
- [ ] Shrinking works correctly for all types
- [ ] Sets maintain uniqueness during shrinking
- [ ] Maps maintain key-value associations during shrinking
- [ ] Documentation explains shrinking behavior for sets/maps
- [ ] Tests cover all new implementations
- [ ] Examples show usage in documentation
