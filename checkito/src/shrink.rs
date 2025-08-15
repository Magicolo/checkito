use crate::{
    generate::Generate,
    state::{Modes, State, States},
};
use core::iter;

/// A trait for types that can be "shrunk" to a simpler value.
///
/// When a property test fails, `checkito` uses the `Shrink` implementation of the
/// failing value's generator to find the smallest possible value that still
/// causes the failure. This process is key to making property testing effective,
/// as it isolates the failure and makes it easier to debug.
///
/// A shrinker is essentially a lazy iterator over simpler versions of a value.
/// Each call to [`Shrink::shrink`] should produce a new shrinker that is "simpler"
/// than the previous one. When `shrink` returns `None`, the value is considered
/// fully shrunk.
///
/// # Implementing `Shrink`
///
/// While `checkito` provides shrinkers for all primitive types and standard
/// collections, you may need to implement it for your own custom types, especially
/// when using custom [`Generate`] implementations.
///
/// The goal is to produce a "simpler" value. For numbers, this means moving
/// closer to zero. For collections, it means making the collection smaller or
/// shrinking its elements.
///
/// # Examples
///
/// A shrinker for a custom `Point` struct that shrinks towards `(0, 0)`:
///
/// ```
/// # use checkito::{Shrink, Generate};
/// #[derive(Clone, Debug, PartialEq)]
/// struct Point {
///     x: i32,
///     y: i32,
/// }
///
/// impl Shrink for Point {
///     type Item = Self;
///
///     fn item(&self) -> Self::Item {
///         self.clone()
///     }
///
///     fn shrink(&mut self) -> Option<Self> {
///         // A simple shrinker that tries to shrink x, then y.
///         // A more advanced shrinker might try shrinking both at once.
///         if self.x != 0 {
///             self.x /= 2;
///             return Some(self.clone());
///         }
///         if self.y != 0 {
///             self.y /= 2;
///             return Some(self.clone());
///         }
///         None
///     }
/// }
/// ```
pub trait Shrink: Clone {
    /// The type of the value that this shrinker produces.
    type Item;
    /// Returns the current value of the shrinker.
    fn item(&self) -> Self::Item;
    /// Produces the next, "simpler" shrinker.
    ///
    /// This method should return `Some(new_shrinker)` if a simpler value can be
    /// produced, or `None` if the value is fully shrunk.
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
