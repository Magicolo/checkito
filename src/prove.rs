use core::convert::Infallible;

pub trait Prove {
    type Proof;
    type Error;
    fn prove(self) -> Result<Self::Proof, Self::Error>;
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
