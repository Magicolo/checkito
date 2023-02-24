use crate::{generate::State, shrink::Shrink, tuples, Generate};
use fastrand::Rng;
use std::{
    error, fmt,
    num::NonZeroUsize,
    thread::{available_parallelism, scope},
};

pub trait IntoCheck {
    fn checker(&self, count: usize) -> Checker<Self>;
}

pub trait Check {
    type Item;

    fn check<P: Prove, F: FnMut(&Self::Item) -> P>(
        &self,
        count: usize,
        check: F,
    ) -> Result<(), Error<Self::Item, P>>;
}

pub trait CheckParallel: Check {
    fn check_parallel<P: Prove + Send + Sync, F: Fn(&Self::Item) -> P + Send + Sync>(
        &self,
        count: usize,
        check: F,
    ) -> Result<(), Error<Self::Item, P>>;
}

pub trait Prove {
    fn prove(&self) -> bool;
}

pub struct Checker<'a, G: ?Sized> {
    pub generator: &'a G,
    pub count: usize,
    pub shrinks: (usize, usize),
    pub seed: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct Error<T, P> {
    pub index: usize,
    pub count: usize,
    pub state: State,
    pub original: (T, P),
    pub shrinks: (usize, usize),
    pub shrunk: Option<(T, P)>,
}

impl<T, P> Error<T, P> {
    pub fn original(&self) -> &T {
        &self.original.0
    }

    pub fn shrunk(&self) -> &T {
        &self.shrunk.as_ref().unwrap_or(&self.original).0
    }
}

impl<'a, G: ?Sized> Checker<'a, G> {
    pub const fn new(generator: &'a G, count: usize) -> Self {
        Self {
            generator,
            count,
            shrinks: (usize::MAX, usize::MAX),
            seed: None,
        }
    }
}

impl<G: ?Sized> Clone for Checker<'_, G> {
    fn clone(&self) -> Self {
        Self {
            generator: self.generator,
            count: self.count,
            shrinks: self.shrinks,
            seed: self.seed,
        }
    }
}

impl<'a, G: Generate + ?Sized> Checker<'a, G> {
    pub fn sequential<'b, P: Prove + 'b, F: FnMut(&G::Item) -> P + 'b>(
        &'b self,
        mut check: F,
    ) -> impl Iterator<Item = Result<G::Item, Error<G::Item, P>>> + 'b {
        let random = self.seed.map_or_else(Rng::new, Rng::with_seed);
        let mut fail = false;
        Iterator::map_while(0..self.count, move |index| {
            if fail {
                return None;
            }

            let mut state = State::new(index, self.count, random.u64(..));
            let (outer_item, mut outer_shrink) = self.generator.generate(&mut state);
            let outer_prove = check(&outer_item);
            if outer_prove.prove() {
                return Some(Ok(outer_item));
            }

            let mut error = Error {
                state,
                index,
                count: self.count,
                original: (outer_item, outer_prove),
                shrinks: (0, 0),
                shrunk: None,
            };

            while error.shrinks.0 < self.shrinks.0 && error.shrinks.1 < self.shrinks.1 {
                if let Some(inner_shrink) = outer_shrink.shrink() {
                    let inner_item = inner_shrink.generate();
                    let inner_prove = check(&inner_item);
                    if inner_prove.prove() {
                        error.shrinks.0 += 1;
                    } else {
                        error.shrinks.0 = 0;
                        error.shrinks.1 += 1;
                        error.shrunk = Some((inner_item, inner_prove));
                        outer_shrink = inner_shrink;
                    }
                } else {
                    break;
                }
            }

            fail = true;
            return Some(Err(error));
        })
    }
}

impl<'a, G: Generate + ?Sized + Send + Sync> Checker<'a, G>
where
    G::Item: Send,
{
    pub fn parallel<'b, P: Prove + Send + Sync + 'b, F: Fn(&G::Item) -> P + Send + Sync + 'b>(
        &'b self,
        check: F,
    ) -> impl Iterator<Item = Result<G::Item, Error<G::Item, P>>> + 'b {
        let parallel = available_parallelism().map_or(1, NonZeroUsize::get);
        let random = self.seed.map_or_else(Rng::new, Rng::with_seed);
        scope(|scope| {
            let mut handles = Vec::new();
            for _ in 0..parallel.min(self.count) {
                let seed = random.u64(..);
                let check = &check;
                let mut checker = self.clone();
                checker.count = (self.count / parallel).max(1);
                checker.seed = Some(seed);
                handles.push(scope.spawn(move || checker.sequential(check).collect::<Vec<_>>()));
            }
            handles
                .into_iter()
                .flat_map(|handle| handle.join())
                .flatten()
                .collect::<Vec<_>>()
                .into_iter()
        })
    }
}

impl<T: fmt::Debug, P: fmt::Debug> fmt::Display for Error<T, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl<T: fmt::Debug, P: fmt::Debug> error::Error for Error<T, P> {}

impl<G: Generate + ?Sized> IntoCheck for G {
    fn checker(&self, count: usize) -> Checker<Self> {
        Checker::new(self, count)
    }
}

impl<G: Generate + ?Sized> Check for G {
    type Item = G::Item;

    fn check<P: Prove, F: FnMut(&G::Item) -> P>(
        &self,
        count: usize,
        check: F,
    ) -> Result<(), Error<Self::Item, P>> {
        for result in self.checker(count).sequential(check) {
            result?;
        }
        Ok(())
    }
}

impl<G: Generate + ?Sized + Send + Sync> CheckParallel for G
where
    G::Item: Send,
{
    fn check_parallel<P: Prove + Send + Sync, F: Fn(&G::Item) -> P + Send + Sync>(
        &self,
        count: usize,
        check: F,
    ) -> Result<(), Error<Self::Item, P>> {
        for result in self.checker(count).parallel(check) {
            result?;
        }
        Ok(())
    }
}

impl Prove for bool {
    fn prove(&self) -> bool {
        *self
    }
}

impl<F: Fn() -> bool> Prove for F {
    fn prove(&self) -> bool {
        self()
    }
}

impl<T, E> Prove for Result<T, E> {
    fn prove(&self) -> bool {
        self.is_ok()
    }
}

impl<P: Prove> Prove for [P] {
    fn prove(&self) -> bool {
        self.iter().all(|proof| proof.prove())
    }
}

impl<P: Prove, const N: usize> Prove for [P; N] {
    fn prove(&self) -> bool {
        self.iter().all(|proof| proof.prove())
    }
}

impl<P: Prove> Prove for Vec<P> {
    fn prove(&self) -> bool {
        self.iter().all(|proof| proof.prove())
    }
}

macro_rules! tuple {
    ($n:ident, $c:tt $(,$p:ident, $t:ident, $i:tt)*) => {
        impl<$($t: Prove,)*> Prove for ($($t,)*) {
            fn prove(&self) -> bool {
                $(self.$i.prove() &&)* true
            }
        }
    };
}

tuples!(tuple);
