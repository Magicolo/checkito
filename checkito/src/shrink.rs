use crate::{
    generate::Generate,
    state::{Modes, State, States},
};
use core::iter;

/// A trait for types that can be "shrunk" to a *smaller* value.
///
/// When a property test fails, `checkito` uses the `Shrink` implementation of
/// the failing value's generator to try to find a smaller value that still
/// causes the failure. This process is key to making property testing
/// effective, as it isolates the failure and makes it easier to debug.
///
/// A shrinker is essentially a lazy iterator over simpler versions of a value.
/// Each call to [`Shrink::shrink`] should produce a new shrinker that is
/// "simpler" than the previous one. When `shrink` returns `None`, the value is
/// considered fully shrunk.
///
/// # Implementing `Shrink`
///
/// While `checkito` provides shrinkers for all primitive types and standard
/// collections, you may need to implement it for your own custom types,
/// especially when using custom [`Generate`] implementations.
///
/// The goal is to produce a *smaller* value (for whatever definition of smaller
/// that makes sense for the item type). For numbers, this means moving
/// closer to zero. For collections, it means making the collection smaller or
/// shrinking its elements.
///
/// # Shrinking Semantics by Type
///
/// Different types have different notions of "smaller":
///
/// - **Numbers**: Shrink toward zero via binary search
///   - `42` → `21` → `10` → `5` → `2` → `1` → `0`
///   - Negative numbers also shrink toward zero: `-42` → `-21` → ... → `0`
///
/// - **Ranges**: Shrink while preserving bounds
///   - A range `10..20` with value `15` shrinks toward the lower bound: `15` → `12` → `10`
///   - Never shrinks below the original range minimum
///
/// - **Collections** (Vec, etc.): Shrink by truncating, then removing elements, then shrinking remaining
///   - `vec![1, 2, 3, 4, 5]` → `vec![1, 2, 3]` → `vec![1, 2]` → `vec![1]` → `vec![]`
///   - After truncation, shrink each element: `vec![42]` → `vec![21]` → `vec![0]`
///
/// - **Strings**: Similar to `Vec<char>`
///   - `"hello"` → `"hel"` → `"he"` → `"h"` → `""`
///
/// - **Booleans**: `true` → `false`
///
/// - **Options**: Shrink the inner value, then unwrap to `None`
///   - `Some(42)` → `Some(21)` → `Some(0)` → `None`
///
/// - **Results**: Shrink the inner value while preserving Ok/Err variant
///   - `Ok(42)` → `Ok(21)` → `Ok(0)`
///
/// # Convergence Requirements
///
/// A shrinking implementation **MUST** eventually return `None` to indicate
/// no further shrinking is possible. This ensures the shrinking process
/// terminates.
///
/// ## Good Implementation
///
/// ```rust
/// # use checkito::shrink::Shrink;
/// #[derive(Clone, Debug)]
/// struct MyInt {
///     value: i32,
/// }
///
/// impl Shrink for MyInt {
///     type Item = i32;
///
///     fn item(&self) -> Self::Item {
///         self.value
///     }
///
///     fn shrink(&mut self) -> Option<Self> {
///         if self.value == 0 {
///             return None;  // Base case - converges!
///         }
///         Some(Self { value: self.value / 2 })
///     }
/// }
/// ```
///
/// ## Bad Implementation (Infinite Loop!)
///
/// ```rust,no_run
/// # use checkito::shrink::Shrink;
/// # #[derive(Clone, Debug)]
/// # struct MyInt { value: i32 }
/// impl Shrink for MyInt {
///     # type Item = i32;
///     # fn item(&self) -> Self::Item { self.value }
///     fn shrink(&mut self) -> Option<Self> {
///         // BUG: Never returns None - infinite shrinking!
///         Some(Self { value: self.value / 2 })
///     }
/// }
/// ```
///
/// The bad implementation will cause infinite shrinking attempts because
/// integer division by 2 eventually reaches 0, but 0/2 is still 0.
///
/// # Preserving Invariants
///
/// Shrinking **MUST** preserve the invariants of the generated value.
/// If your type has constraints, shrinking must respect them.
///
/// ## Example: Range-Constrained Values
///
/// ```rust
/// # use checkito::shrink::Shrink;
/// #[derive(Clone, Debug)]
/// struct PositiveInt(i32); // Invariant: value > 0
///
/// impl Shrink for PositiveInt {
///     type Item = i32;
///
///     fn item(&self) -> Self::Item {
///         self.0
///     }
///
///     fn shrink(&mut self) -> Option<Self> {
///         if self.0 <= 1 {
///             return None;  // Can't shrink below 1 - preserves invariant
///         }
///         Some(Self(self.0 / 2))
///     }
/// }
/// ```
///
/// ## Example: Non-Empty Collections
///
/// ```rust
/// # use checkito::shrink::Shrink;
/// #[derive(Clone, Debug)]
/// struct NonEmptyVec<T: Clone>(Vec<T>); // Invariant: len() > 0
///
/// impl<T: Clone> Shrink for NonEmptyVec<T> {
///     type Item = Vec<T>;
///
///     fn item(&self) -> Self::Item {
///         self.0.clone()
///     }
///
///     fn shrink(&mut self) -> Option<Self> {
///         if self.0.len() == 1 {
///             return None;  // Can't remove the last element
///         }
///         // Remove elements but keep at least one
///         let mut smaller = self.0.clone();
///         smaller.pop();
///         Some(Self(smaller))
///     }
/// }
/// ```
///
/// # Relationship with `Generate`
///
/// A shrinker is created during generation and represents the "shrink space" -
/// all possible smaller values that could be explored if the test fails.
///
/// The shrinker respects the original generator's constraints. For example,
/// if generated from a `Range(10..20)`, shrinking should never produce values
/// outside that range.
///
/// # Example: Custom Type
///
/// ```rust
/// use checkito::shrink::Shrink;
///
/// #[derive(Debug, Clone)]
/// struct Person {
///     name: String,
///     age: u8,
/// }
///
/// // A simple shrinker that tries to shrink the name and age independently
/// impl Shrink for Person {
///     type Item = Person;
///
///     fn item(&self) -> Self::Item {
///         self.clone()
///     }
///     
///     fn shrink(&mut self) -> Option<Self> {
///         // Strategy 1: Try to shrink the name (remove characters)
///         if !self.name.is_empty() {
///             let mut shorter_name = self.name.clone();
///             shorter_name.pop();
///             return Some(Person {
///                 name: shorter_name,
///                 age: self.age,
///             });
///         }
///         
///         // Strategy 2: Try to shrink age toward 0
///         if self.age > 0 {
///             return Some(Person {
///                 name: self.name.clone(),
///                 age: self.age / 2,
///             });
///         }
///         
///         // No more shrinking possible
///         None
///     }
/// }
/// ```
///
/// # Testing Shrink Implementations
///
/// When implementing `Shrink`, test that:
/// 1. Shrinking eventually returns `None` (convergence)
/// 2. Shrunken values are actually smaller (progress)
/// 3. Invariants are preserved (correctness)
///
/// ```rust
/// # use checkito::shrink::Shrink;
/// # #[derive(Clone, Debug, PartialEq)]
/// # struct PositiveInt(i32);
/// # impl Shrink for PositiveInt {
/// #     type Item = i32;
/// #     fn item(&self) -> Self::Item { self.0 }
/// #     fn shrink(&mut self) -> Option<Self> {
/// #         if self.0 <= 1 { None } else { Some(Self(self.0 / 2)) }
/// #     }
/// # }
/// #[test]
/// fn shrink_converges() {
///     let mut current = PositiveInt(100);
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
///     let mut current = PositiveInt(42);
///     
///     while let Some(smaller) = current.shrink() {
///         assert!(smaller.0 > 0, "Invariant violated!");
///         current = smaller;
///     }
/// }
/// ```
///
/// # Examples
///
/// Basic shrinking demonstration:
///
/// ```rust
/// use checkito::shrink::Shrink;
/// 
/// #[derive(Clone, Debug)]
/// struct Counter(i32);
///
/// impl Shrink for Counter {
///     type Item = i32;
///
///     fn item(&self) -> Self::Item {
///         self.0
///     }
///
///     fn shrink(&mut self) -> Option<Self> {
///         if self.0 == 0 {
///             None
///         } else {
///             Some(Self(self.0 / 2))
///         }
///     }
/// }
///
/// let mut counter = Counter(100);
/// assert_eq!(counter.item(), 100);
///
/// // Shrink once
/// let mut smaller = counter.shrink().unwrap();
/// assert_eq!(smaller.item(), 50);
///
/// // Keep shrinking
/// smaller = smaller.shrink().unwrap();
/// assert_eq!(smaller.item(), 25);
/// ```
pub trait Shrink: Clone {
    /// The type of the item being shrunk.
    type Item;
    
    /// Returns the current item represented by this shrinker.
    fn item(&self) -> Self::Item;
    
    /// Returns a shrinker for a *smaller* value, or `None` if fully shrunk.
    ///
    /// This method should return `Some(shrinker)` if a *smaller* value can be
    /// produced, or `None` if the value is fully shrunk and cannot be simplified further.
    ///
    /// See the trait documentation for detailed shrinking semantics and requirements.
    fn shrink(&mut self) -> Option<Self>;
}

#[derive(Debug, Clone)]
pub struct Shrinker<T: ?Sized>(pub(crate) T);

#[derive(Debug, Clone)]
pub(crate) struct Shrinkers<G: ?Sized> {
    states: States,
    generator: G,
}

impl<G: Generate + ?Sized> Generate for Shrinker<G> {
    type Item = G::Shrink;
    type Shrink = Shrinker<G::Shrink>;

    const CARDINALITY: Option<u128> = G::CARDINALITY;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Shrinker(self.0.generate(state))
    }

    fn cardinality(&self) -> Option<u128> {
        self.0.cardinality()
    }
}

impl<S: Shrink> Shrink for Shrinker<S> {
    type Item = S;

    fn item(&self) -> Self::Item {
        self.0.clone()
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self(self.0.shrink()?))
    }
}

impl<G: Generate> Shrinkers<G> {
    pub(crate) fn new(generator: G, modes: Modes) -> Self {
        Shrinkers {
            generator,
            states: modes.into(),
        }
    }
}

impl<G: Generate + ?Sized> Iterator for Shrinkers<G> {
    type Item = G::Shrink;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.next()?))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.states.size_hint()
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.states.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.nth(n)?))
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        Some(self.generator.generate(&mut self.states.last()?))
    }
}

impl<G: Generate + ?Sized> DoubleEndedIterator for Shrinkers<G> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.next_back()?))
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.nth_back(n)?))
    }
}

impl<G: Generate + ?Sized> ExactSizeIterator for Shrinkers<G> {
    fn len(&self) -> usize {
        self.states.len()
    }
}

impl<G: Generate + ?Sized> iter::FusedIterator for Shrinkers<G> {}
