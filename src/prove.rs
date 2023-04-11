use std::{error, fmt};

use crate::tuples;

pub trait Prove {
    fn prove(&self) -> bool;
    fn is(&self, prove: &Self) -> bool;
}

#[derive(Clone, Copy, PartialEq, Eq)]
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

    fn is(&self, prove: &Self) -> bool {
        self == prove
    }
}

impl<T: Eq, E: Eq> Prove for Result<T, E> {
    fn prove(&self) -> bool {
        self.is_ok()
    }

    fn is(&self, prove: &Self) -> bool {
        self == prove
    }
}

impl<P: Prove + Eq> Prove for [P] {
    fn prove(&self) -> bool {
        self.iter().all(|proof| proof.prove())
    }

    fn is(&self, prove: &Self) -> bool {
        self == prove
    }
}

impl<P: Prove + Eq, const N: usize> Prove for [P; N] {
    fn prove(&self) -> bool {
        self.iter().all(|proof| proof.prove())
    }

    fn is(&self, prove: &Self) -> bool {
        self == prove
    }
}

impl<P: Prove + Eq> Prove for Vec<P> {
    fn prove(&self) -> bool {
        self.iter().all(|proof| proof.prove())
    }

    fn is(&self, prove: &Self) -> bool {
        self == prove
    }
}

macro_rules! tuple {
    ($n:ident, $c:tt $(,$p:ident, $t:ident, $i:tt)*) => {
        impl<$($t: Prove + Eq,)*> Prove for ($($t,)*) {
            fn prove(&self) -> bool {
                $(self.$i.prove() &&)* true
            }

            fn is(&self, _prove: &Self) -> bool {
                $(self.$i == _prove.$i &&)* true
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
