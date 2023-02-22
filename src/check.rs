use crate::{generate::State, shrink::Shrink, tuples, Generate};
use fastrand::Rng;
use std::{error, fmt, thread::scope};

pub trait Check<T> {
    #[inline]
    fn check<P: Prove, F: FnMut(&T) -> P>(
        &self,
        count: usize,
        check: F,
    ) -> Result<(), Error<P, T>> {
        self.check_with(count, None, check)
    }

    fn check_with<P: Prove, F: FnMut(&T) -> P>(
        &self,
        count: usize,
        seed: Option<u64>,
        check: F,
    ) -> Result<(), Error<P, T>>;
}

pub trait CheckParallel<T> {
    #[inline]
    fn check_parallel<P: Prove + Send + Sync, F: Fn(&T) -> P + Send + Sync>(
        &self,
        count: usize,
        check: F,
    ) -> Result<(), Error<P, T>> {
        self.check_parallel_with(count, None, check)
    }

    fn check_parallel_with<P: Prove + Send + Sync, F: Fn(&T) -> P + Send + Sync>(
        &self,
        count: usize,
        seed: Option<u64>,
        check: F,
    ) -> Result<(), Error<P, T>>;
}

pub trait Prove {
    fn prove(&self) -> bool;
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

impl<P: fmt::Debug, T: fmt::Debug> fmt::Display for Error<P, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl<P: fmt::Debug, T: fmt::Debug> error::Error for Error<P, T> {}

impl<G: Generate + ?Sized> Check<G::Item> for G {
    fn check_with<P: Prove, F: FnMut(&G::Item) -> P>(
        &self,
        count: usize,
        seed: Option<u64>,
        mut check: F,
    ) -> Result<(), Error<P, G::Item>> {
        let random = seed.map_or_else(Rng::new, Rng::with_seed);
        for index in 0..count {
            let mut state = State::new(index, count, random.u64(..));
            let (outer_item, mut outer_shrink) = self.generate(&mut state);
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

impl<G: Generate + ?Sized + Send + Sync> CheckParallel<G::Item> for G
where
    G::Item: Send,
{
    fn check_parallel_with<P: Prove + Send + Sync, F: Fn(&G::Item) -> P + Send + Sync>(
        &self,
        count: usize,
        seed: Option<u64>,
        check: F,
    ) -> Result<(), Error<P, G::Item>> {
        let parallel = num_cpus::get();
        let random = seed.map_or_else(Rng::new, Rng::with_seed);
        let results: Vec<_> =
            scope(|scope| {
                let mut handles = Vec::new();
                for _ in 0..parallel.min(count) {
                    let seed = random.u64(..);
                    let check = &check;
                    handles.push(scope.spawn(move || {
                        self.check_with((count / parallel).max(1), Some(seed), check)
                    }));
                }
                handles.into_iter().map(|handle| handle.join()).collect()
            });
        for result in results {
            result.unwrap()?;
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
