use crate::{
    Generator, boxed, check,
    generate::{State, States},
    random,
    utility::tuples,
};
use core::{iter, ops};

#[derive(Clone, Debug)]
pub struct All<T: ?Sized> {
    pub index: usize,
    pub items: T,
}

pub trait Shrinker: Clone {
    type Item;

    fn item(&self) -> Self::Item;
    fn shrink(&mut self) -> Option<Self>;

    fn boxed(self) -> boxed::Shrink<Self::Item>
    where
        Self: 'static,
    {
        boxed::Shrink::new(self)
    }
}

#[derive(Debug)]
pub struct Shrinkers<'a, G: ?Sized> {
    generator: &'a G,
    states: States,
}

#[derive(Debug, Clone)]
pub struct Shrink<T: ?Sized>(pub T);

impl<G: Generator + ?Sized> Generator for Shrink<G> {
    type Item = G::Shrink;
    type Shrink = Shrink<G::Shrink>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Shrink(self.0.generate(state))
    }
}

impl<S: Shrinker> Shrinker for Shrink<S> {
    type Item = S;

    fn item(&self) -> Self::Item {
        self.0.clone()
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self(self.0.shrink()?))
    }
}

impl<G: Generator + ?Sized> Clone for Shrinkers<'_, G> {
    fn clone(&self) -> Self {
        Self {
            generator: self.generator,
            states: self.states.clone(),
        }
    }
}

impl<'a, G: Generator + ?Sized> From<&'a G> for Shrinkers<'a, G> {
    fn from(value: &'a G) -> Self {
        Shrinkers::new(value, check::COUNT, 0.0..1.0, None)
    }
}

impl<'a, G: Generator + ?Sized> Shrinkers<'a, G> {
    pub fn new(generator: &'a G, count: usize, size: ops::Range<f64>, seed: Option<u64>) -> Self {
        Shrinkers {
            generator,
            states: States::new(count, size, seed),
        }
    }
}

pub(crate) fn shrinker<G: Generator + ?Sized>(
    generator: &G,
    size: f64,
    seed: Option<u64>,
) -> G::Shrink {
    let mut state = State::new(0, 1, size..size, seed.unwrap_or_else(random::seed));
    generator.generate(&mut state)
}

impl<T> All<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            items: inner,
            index: 0,
        }
    }
}

impl<G: Generator + ?Sized> Iterator for Shrinkers<'_, G> {
    type Item = G::Shrink;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.next()?))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.states.size_hint()
    }

    fn count(self) -> usize {
        self.states.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.nth(n)?))
    }

    fn last(self) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.last()?))
    }
}

impl<G: Generator + ?Sized> DoubleEndedIterator for Shrinkers<'_, G> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.next_back()?))
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        Some(self.generator.generate(&mut self.states.nth_back(n)?))
    }
}

impl<G: Generator + ?Sized> ExactSizeIterator for Shrinkers<'_, G> {
    fn len(&self) -> usize {
        self.states.len()
    }
}

impl<G: Generator + ?Sized> iter::FusedIterator for Shrinkers<'_, G> {}

macro_rules! tuple {
    ($n:ident, $c:tt $(,$p:ident, $t:ident, $i:tt)*) => {
        impl<$($t: Shrinker,)*> Shrinker for All<($($t,)*)> {
            type Item = ($($t::Item,)*);

            #[allow(clippy::unused_unit)]
            fn item(&self) -> Self::Item {
                ($(self.items.$i.item(),)*)
            }

            fn shrink(&mut self) -> Option<Self> {
                loop {
                    match self.index {
                        $($i => {
                            if let Some(shrinker) = self.items.$i.shrink() {
                                let mut items = self.items.clone();
                                items.$i = shrinker;
                                break Some(Self { items, index: self.index });
                            } else {
                                self.index += 1;
                            }
                        })*
                        _ => break None,
                    }
                }
            }
        }
    };
}

tuples!(tuple);
