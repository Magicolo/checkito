use crate::{generate::State, shrink::Shrink, tuples, Generate};
use fastrand::Rng;
use std::{error, fmt, marker::PhantomData, thread::scope};

pub trait IntoCheck {
    fn checker(&self) -> Checker<Self>;
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
    generator: &'a G,
    shrinks: (usize, usize),
    seed: Option<u64>,
}

pub struct Checkez<'a, G: ?Sized, C, P> {
    generator: &'a G,
    index: usize,
    count: usize,
    shrinks: (usize, usize),
    random: Rng,
    check: C,
    _marker: PhantomData<P>,
}

#[derive(Clone, Debug)]
pub struct Error<T, P> {
    pub index: usize,
    pub state: State,
    pub original: (T, P),
    pub count: usize,
    pub shrinks: (usize, usize),
    pub shrunk: Option<(T, P)>,
}

impl<T, P> Error<T, P> {
    pub fn original(&self) -> &T {
        &self.original.0
    }

    // pub fn shrink(&mut self) -> bool {
    //     while let Some(shrink) = self.shrinker.shrink() {
    //         let item = shrink.generate();
    //         let prove = (self.check)(&item);
    //         if prove.prove() {
    //             continue;
    //         } else {
    //             self.shrunk = Some((item, prove));
    //             self.shrinker = shrink;
    //             return true;
    //         }
    //     }
    //     false
    // }

    pub fn shrunk(&self) -> &T {
        &self.shrunk.as_ref().unwrap_or(&self.original).0
    }
}

impl<'a, G: ?Sized> Checker<'a, G> {
    pub const fn new(generator: &'a G, shrinks: (usize, usize), seed: Option<u64>) -> Self {
        Self {
            generator,
            shrinks,
            seed,
        }
    }

    pub fn with_shrinks(mut self, shrinks: (usize, usize)) -> Self {
        self.shrinks = shrinks;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }
}

impl<'a, G: Generate + ?Sized, C: FnMut(&G::Item) -> P, P: Prove> Iterator
    for Checkez<'a, G, C, P>
{
    type Item = Result<G::Item, Error<G::Item, P>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }

        let mut state = State::new(self.index, self.count, self.random.u64(..));
        let (outer_item, mut outer_shrink) = self.generator.generate(&mut state);
        let outer_prove = (self.check)(&outer_item);
        if outer_prove.prove() {
            Some(Ok(outer_item))
        } else {
            let mut error = Error {
                state,
                index: self.index,
                count: self.count,
                original: (outer_item, outer_prove),
                shrinks: (0, 0),
                shrunk: None,
            };
            while error.shrinks.0 < self.shrinks.0 && error.shrinks.1 < self.shrinks.1 {
                if let Some(inner_shrink) = outer_shrink.shrink() {
                    let inner_item = inner_shrink.generate();
                    let inner_prove = (self.check)(&inner_item);
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
            Some(Err(error))
        }
    }
}

impl<T: fmt::Debug, P: fmt::Debug> fmt::Display for Error<T, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl<T: fmt::Debug, P: fmt::Debug> error::Error for Error<T, P> {}

impl<G: Generate + ?Sized> Check for Checker<'_, G> {
    type Item = G::Item;

    fn check<P: Prove, F: FnMut(&G::Item) -> P>(
        &self,
        count: usize,
        mut check: F,
    ) -> Result<(), Error<Self::Item, P>> {
        let random = self.seed.map_or_else(Rng::new, Rng::with_seed);
        for index in 0..count {
            let mut state = State::new(index, count, random.u64(..));
            let (outer_item, mut outer_shrink) = self.generator.generate(&mut state);
            let outer_prove = check(&outer_item);
            if outer_prove.prove() {
                continue;
            }

            let mut error = Error {
                state,
                index,
                count,
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
            return Err(error);
        }
        Ok(())
    }
}

impl<G: Generate + ?Sized + Send + Sync> CheckParallel for Checker<'_, G>
where
    G::Item: Send,
{
    fn check_parallel<P: Prove + Send + Sync, F: Fn(&G::Item) -> P + Send + Sync>(
        &self,
        count: usize,
        check: F,
    ) -> Result<(), Error<Self::Item, P>> {
        let parallel = num_cpus::get().max(1);
        let random = self.seed.map_or_else(Rng::new, Rng::with_seed);
        let results: Vec<_> = scope(|scope| {
            let mut handles = Vec::new();
            for _ in 0..parallel.min(count) {
                let seed = random.u64(..);
                let check = &check;
                let checker = Checker::new(self.generator, self.shrinks, Some(seed));
                handles.push(scope.spawn(move || checker.check((count / parallel).max(1), check)));
            }
            handles.into_iter().map(|handle| handle.join()).collect()
        });
        for result in results {
            result.unwrap()?;
        }
        Ok(())
    }
}

impl<'a, G: Generate + ?Sized> From<&'a G> for Checker<'a, G> {
    fn from(value: &'a G) -> Self {
        Self::new(value, (usize::MAX, usize::MAX), None)
    }
}

impl<G: Generate + ?Sized> IntoCheck for G {
    fn checker(&self) -> Checker<Self> {
        Checker::from(self)
    }
}

impl<G: Generate + ?Sized> Check for G {
    type Item = G::Item;

    fn check<P: Prove, F: FnMut(&G::Item) -> P>(
        &self,
        count: usize,
        check: F,
    ) -> Result<(), Error<Self::Item, P>> {
        Checker::from(self).check(count, check)
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
        Checker::from(self).check_parallel(count, check)
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
