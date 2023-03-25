use std::{error, fmt};

use crate::tuples;

pub trait Prove {
    fn prove(&self) -> bool;
}

#[derive(Clone, Copy)]
pub struct Error<P> {
    pub name: &'static str,
    pub prove: P,
}

impl<P> fmt::Debug for Error<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl<P> fmt::Display for Error<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.name, f)
    }
}

impl<P> error::Error for Error<P> {}

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

#[macro_export]
macro_rules! prove {
    ($prove:expr) => {{
        let prove = $prove;
        if $crate::prove::Prove::prove(&prove) {
            Ok(stringify!($prove))
        } else {
            Err($crate::prove::Error { name: stringify!($prove), prove })
        }
    }};
    ($($prove:expr),*) => { Ok(($($crate::prove!($prove)),*)) }
}
