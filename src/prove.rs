use std::{error, fmt};

use crate::tuples;

pub trait Prove {
    fn prove(&self) -> bool;
    fn is(&self, prove: &Self) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error<P> {
    pub prove: P,
    pub expression: &'static str,
    pub file: &'static str,
    pub module: &'static str,
    pub line: u32,
    pub column: u32,
}

impl<P: fmt::Debug> fmt::Display for Error<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<P: fmt::Debug> error::Error for Error<P> {}

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
            Ok(prove)
        } else {
            Err($crate::prove::Error {
                prove,
                expression: stringify!($prove),
                file: file!(),
                line: line!(),
                column: column!(),
                module: module_path!(),
            })
        }
    }};
    ($($prove:expr),*) => { Ok(($($crate::prove!($prove)),*)) }
}
