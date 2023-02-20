use crate::{generate::State, shrink::Shrink, tuples, Generate};
use fastrand::Rng;

pub trait Check<T> {
    fn check<P: Proof, F: Fn(&T) -> P>(
        &self,
        count: usize,
        seed: Option<u64>,
        check: F,
    ) -> Result<(), Error<P, T>>;
}

pub trait Proof {
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

impl<G: Generate + ?Sized> Check<G::Item> for G {
    fn check<P: Proof, F: Fn(&G::Item) -> P>(
        &self,
        count: usize,
        seed: Option<u64>,
        check: F,
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

impl Proof for bool {
    fn prove(&self) -> bool {
        *self
    }
}

impl<T, E> Proof for Result<T, E> {
    fn prove(&self) -> bool {
        self.is_ok()
    }
}

macro_rules! tuple {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Proof,)*> Proof for ($($t,)*) {
            fn prove(&self) -> bool {
                let ($($p,)*) = self;
                $($p.prove() &&)* true
            }
        }
    };
}

tuples!(tuple);
