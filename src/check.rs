use crate::{generate::State, shrink::Shrink, tuples, Generate};
use fastrand::Rng;
use std::{error, fmt, thread::scope};

pub trait IntoCheck {
    fn checker(&self, seed: Option<u64>) -> Checker<Self>;
}

pub trait Check {
    type Item;
    fn check<P: Prove, F: FnMut(&Self::Item) -> P>(
        &self,
        count: usize,
        check: F,
    ) -> Result<(), Error<P, Self::Item>>;
}

pub trait CheckParallel: Check {
    fn check_parallel<P: Prove + Send + Sync, F: Fn(&Self::Item) -> P + Send + Sync>(
        &self,
        count: usize,
        check: F,
    ) -> Result<(), Error<P, Self::Item>>;
}

pub trait Prove {
    fn prove(&self) -> bool;
}

pub struct Checker<'a, G: ?Sized> {
    seed: Option<u64>,
    generator: &'a G,
}

#[derive(Clone, Debug)]
pub struct Error<P, T> {
    pub index: usize,
    pub count: usize,
    pub state: State,
    pub original: (T, P),
    pub shrunk: Option<(T, P)>,
}

impl<P, T> Error<P, T> {
    pub fn original(&self) -> &T {
        &self.original.0
    }

    pub fn shrunk(&self) -> &T {
        &self.shrunk.as_ref().unwrap_or(&self.original).0
    }
}

impl<'a, G: ?Sized> Checker<'a, G> {
    pub const fn new(generator: &'a G, seed: Option<u64>) -> Self {
        Self { generator, seed }
    }
}

impl<P: fmt::Debug, T: fmt::Debug> fmt::Display for Error<P, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl<P: fmt::Debug, T: fmt::Debug> error::Error for Error<P, T> {}

impl<G: Generate + ?Sized> Check for Checker<'_, G> {
    type Item = G::Item;

    fn check<P: Prove, F: FnMut(&G::Item) -> P>(
        &self,
        count: usize,
        mut check: F,
    ) -> Result<(), Error<P, G::Item>> {
        let random = self.seed.map_or_else(Rng::new, Rng::with_seed);
        for index in 0..count {
            let mut state = State::new(index, count, random.u64(..));
            let (outer_item, mut outer_shrink) = self.generator.generate(&mut state);
            let outer_proof = check(&outer_item);
            if outer_proof.prove() {
                continue;
            }

            let mut error = Error {
                state,
                index,
                count,
                original: (outer_item, outer_proof),
                shrunk: None,
            };
            while let Some(inner_shrink) = outer_shrink.shrink() {
                let inner_item = inner_shrink.generate();
                let inner_proof = check(&inner_item);
                if inner_proof.prove() {
                    continue;
                }

                error.shrunk = Some((inner_item, inner_proof));
                outer_shrink = inner_shrink;
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
    ) -> Result<(), Error<P, G::Item>> {
        let parallel = num_cpus::get().max(1);
        let random = self.seed.map_or_else(Rng::new, Rng::with_seed);
        let results: Vec<_> = scope(|scope| {
            let mut handles = Vec::new();
            for _ in 0..parallel.min(count) {
                let seed = random.u64(..);
                let check = &check;
                let checker = Checker::new(self.generator, Some(seed));
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
        Self::new(value, None)
    }
}

impl<G: Generate + ?Sized> IntoCheck for G {
    fn checker(&self, seed: Option<u64>) -> Checker<Self> {
        Checker::new(self, seed)
    }
}

impl<G: Generate + ?Sized> Check for G {
    type Item = G::Item;

    fn check<P: Prove, F: FnMut(&G::Item) -> P>(
        &self,
        count: usize,
        check: F,
    ) -> Result<(), Error<P, G::Item>> {
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
    ) -> Result<(), Error<P, G::Item>> {
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
