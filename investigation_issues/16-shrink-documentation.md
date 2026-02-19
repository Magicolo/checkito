# Missing Documentation for Shrink Trait and Shrinking Strategies

## Summary
The `Shrink` trait in `shrink.rs` lacks comprehensive documentation explaining shrinking semantics, strategies, and expectations for custom implementations. This makes it difficult for users to implement `Shrink` for their types correctly.

## Context
Shrinking is a core feature of property testing - when a test fails, the library tries to find the "smallest" input that still fails. Understanding shrinking semantics is critical for:
1. Implementing custom shrinking for user types
2. Understanding test results and minimal failing cases
3. Debugging why certain values shrink differently

## Current Documentation State

**Location**: `checkito/src/shrink.rs:7-35`

**What's Documented**:
- Basic trait structure (lines 7-35)
- `shrink(&mut self) -> Option<Self>` method signature
- Brief comment: "Returns a 'smaller' version" (line 34)

**What's Missing**:
- What does "smaller" mean for different types?
- When should `shrink()` return `None`?
- How to ensure shrinking converges?
- Examples of good vs bad shrinking implementations
- Relationship between shrinking and `Generate` bounds
- How shrinking interacts with composition (`Map`, `Filter`, etc.)

## Missing Documentation Topics

### 1. Shrinking Semantics for Built-in Types
**Should document**:
```rust
/// # Shrinking Semantics by Type
///
/// Different types have different notions of "smaller":
///
/// - **Numbers**: Shrink toward zero
///   - `42` → `21` → `10` → `5` → `2` → `1` → `0`
///   - Negative numbers shrink toward zero: `-42` → `-21` → ... → `0`
///
/// - **Ranges**: Shrink while preserving bounds
///   - Range `10..20` with value `15` → `12` → `10` (lower bound)
///   - Never shrinks below the original range minimum
///
/// - **Collections**: Shrink by removing elements, then shrinking remaining
///   - `vec![1, 2, 3]` → `vec![1, 2]` → `vec![1]` → `vec![]`
///   - After truncation, shrink each element: `vec![42]` → `vec![21]` → `vec![0]`
///
/// - **Strings**: Similar to `Vec<char>`
///   - `"hello"` → `"hell"` → `"hel"` → `"he"` → `"h"` → `""`
///
/// - **Options**: `Some(x)` → `Some(shrink(x))` → `None`
///
/// - **Results**: `Ok(x)` → `Ok(shrink(x))`, `Err(e)` → `Err(shrink(e))`
```

### 2. Convergence Requirements
**Should document**:
```rust
/// # Convergence Requirements
///
/// A shrinking implementation MUST eventually return `None` to indicate
/// no further shrinking is possible. This ensures the shrinking process
/// terminates.
///
/// ## Good Implementation
/// ```rust
/// impl Shrink for MyInt {
///     type Shrink = Self;
///     fn shrink(&self) -> Self::Shrink {
///         if self.value == 0 {
///             return None;  // Base case!
///         }
///         Some(Self { value: self.value / 2 })
///     }
/// }
/// ```
///
/// ## Bad Implementation (Infinite Loop!)
/// ```rust
/// impl Shrink for MyInt {
///     type Shrink = Self;
///     fn shrink(&self) -> Self::Shrink {
///         // BUG: Never returns None!
///         Some(Self { value: self.value / 2 })
///     }
/// }
/// ```
///
/// The bad implementation will cause infinite shrinking attempts.
```

### 3. Preserving Invariants
**Should document**:
```rust
/// # Preserving Invariants
///
/// Shrinking MUST preserve the invariants of the generated value.
///
/// ## Example: Range-Constrained Values
/// ```rust
/// struct PositiveInt(i32); // Invariant: value > 0
///
/// impl Shrink for PositiveInt {
///     type Shrink = Self;
///     fn shrink(&self) -> Self::Shrink {
///         if self.0 <= 1 {
///             return None;  // Can't shrink below 1
///         }
///         Some(Self(self.0 / 2))
///     }
/// }
/// ```
///
/// ## Example: Non-Empty Collections
/// ```rust
/// struct NonEmptyVec<T>(Vec<T>); // Invariant: len() > 0
///
/// impl<T: Shrink> Shrink for NonEmptyVec<T> {
///     type Shrink = Self;
///     fn shrink(&self) -> Self::Shrink {
///         if self.0.len() == 1 {
///             // Shrink the element instead
///             return self.0[0].shrink().map(|x| Self(vec![x]));
///         }
///         // Remove elements but keep at least one
///         let mut smaller = self.0.clone();
///         smaller.pop();
///         Some(Self(smaller))
///     }
/// }
/// ```
```

### 4. Interaction with Generate
**Should document**:
```rust
/// # Relationship with `Generate`
///
/// A shrinker is created during generation and attached to the generated value.
/// The shrinker represents the "shrink space" - all possible smaller values.
///
/// ```rust
/// let mut state = State::default();
/// let shrinker = (0..100).generate(&mut state);
///
/// // Get current value
/// let value = shrinker.item().unwrap(); // e.g., 42
///
/// // Get smaller values
/// let smaller = shrinker.shrink(); // e.g., 21
/// let even_smaller = smaller.shrink(); // e.g., 10
/// ```
///
/// **Important**: The shrinker must respect the original generator's constraints.
/// For example, if generated from `Range(10..20)`, shrinking should never
/// produce values outside that range.
```

### 5. Custom Type Example
**Should include full example**:
```rust
/// # Example: Custom Type
///
/// ```rust
/// use checkito::*;
///
/// #[derive(Debug, Clone)]
/// struct Person {
///     name: String,
///     age: u8,
/// }
///
/// impl Shrink for Person {
///     type Shrink = Self;
///     
///     fn shrink(&self) -> Self::Shrink {
///         // Strategy: Shrink name and age independently
///         if let Some(shorter_name) = self.name.shrink() {
///             return Some(Person {
///                 name: shorter_name.item()?,
///                 age: self.age,
///             });
///         }
///         
///         if let Some(younger) = self.age.shrink() {
///             return Some(Person {
///                 name: self.name.clone(),
///                 age: younger.item()?,
///             });
///         }
///         
///         None
///     }
/// }
/// ```
```

## Comment Style Issue

**Location**: `checkito/src/shrink.rs:34`

**Current**:
```rust
fn shrink(&mut self) -> Option<Self> // Returns a 'smaller' version of itself.
```

**Problem**: Documentation is a comment AFTER the signature, not a doc comment BEFORE.

**Should be**:
```rust
/// Returns a 'smaller' version of itself.
///
/// See trait documentation for shrinking semantics.
fn shrink(&mut self) -> Option<Self>
```

## Missing Examples in Docstrings

The trait itself has no `# Examples` section showing actual usage.

**Should add**:
```rust
/// # Examples
///
/// Shrinking a number:
/// ```rust
/// use checkito::*;
///
/// let mut shrinker = 42i32.into_shrink();
/// assert_eq!(shrinker.item(), Some(42));
///
/// let smaller = shrinker.shrink();
/// assert!(smaller.item().unwrap() < 42);
/// ```
///
/// Shrinking a vector:
/// ```rust
/// use checkito::*;
///
/// let vec = vec![1, 2, 3, 4, 5];
/// let mut shrinker = vec.into_shrink();
///
/// // First shrinks by removing elements
/// let smaller = shrinker.shrink();
/// assert!(smaller.item().unwrap().len() < 5);
///
/// // Then shrinks individual elements
/// // ... continues until vec![] or no further shrinking possible
/// ```
```

## Testing Documentation

**Missing**: No examples showing how to test custom `Shrink` implementations.

**Should add**:
```rust
/// # Testing Shrink Implementations
///
/// When implementing `Shrink`, test that:
/// 1. Shrinking eventually returns `None`
/// 2. Shrunken values are actually smaller
/// 3. Invariants are preserved
///
/// ```rust
/// #[test]
/// fn shrink_converges() {
///     let mut current = MyType::new(100);
///     let mut count = 0;
///     
///     while let Some(smaller) = current.shrink() {
///         current = smaller;
///         count += 1;
///         assert!(count < 1000, "Shrinking didn't converge!");
///     }
/// }
///
/// #[test]
/// fn shrink_preserves_invariants() {
///     let original = PositiveInt(42);
///     let mut current = original.clone();
///     
///     while let Some(smaller) = current.shrink() {
///         assert!(smaller.0 > 0, "Invariant violated!");
///         current = smaller;
///     }
/// }
/// ```
```

## Priority
**High** - The `Shrink` trait is a core public API, and users need clear guidance to implement it correctly.

## Related Issues
- Issue #8: "Add doc examples/tests in main traits"
- This is specifically for the `Shrink` trait

## Acceptance Criteria
- [ ] Comprehensive trait-level documentation
- [ ] Shrinking semantics explained for all built-in types
- [ ] Convergence requirements documented
- [ ] Invariant preservation explained with examples
- [ ] Full custom type example
- [ ] Testing guidance for custom implementations
- [ ] Fix comment style (use doc comments)
- [ ] Add multiple `# Examples` sections
- [ ] Cross-reference with `Generate` trait
- [ ] Examples compile and pass doctests
