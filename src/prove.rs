use core::{convert::Infallible, error, fmt};

pub trait Prove {
    type Proof;
    type Error;
    fn prove(self) -> Result<Self::Proof, Self::Error>;
}

#[derive(Clone, Debug)]
pub struct Error {
    pub values: Vec<String>,
    pub expression: &'static str,
    pub file: &'static str,
    pub module: &'static str,
    pub line: u32,
    pub column: u32,
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl Prove for () {
    type Error = Infallible;
    type Proof = ();

    fn prove(self) -> Result<Self::Proof, Self::Error> {
        Ok(())
    }
}

impl Prove for bool {
    type Error = ();
    type Proof = ();

    fn prove(self) -> Result<Self::Proof, Self::Error> {
        if self { Ok(()) } else { Err(()) }
    }
}

impl<T, E> Prove for Result<T, E> {
    type Error = E;
    type Proof = T;

    fn prove(self) -> Self {
        self
    }
}

impl Prove for Error {
    type Error = Self;
    type Proof = Infallible;

    fn prove(self) -> Result<Self::Proof, Self::Error> {
        Err(self)
    }
}

#[macro_export]
macro_rules! prove {
    ([$($values: expr),*] $prove:expr) => {{
        match $crate::prove::Prove::prove($prove) {
            Ok(proof) => proof,
            Err(error) => return Err($crate::prove::Error {
                values: vec![$(format!("{:?}", $value)),*],
                expression: stringify!($prove),
                file: file!(),
                line: line!(),
                column: column!(),
                module: module_path!(),
            });
        }
    }};
    ($prove:expr) => { $crate::prove!([] $prove) };
    ($($prove:expr),*) => { Ok(($($crate::prove!($prove)),*)) }
}
