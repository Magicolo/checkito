use std::{error, fmt};

pub trait Prove {
    fn prove(&self) -> bool;
}

pub struct Proof<P>(&'static str, P);

impl<P> fmt::Debug for Proof<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<P> fmt::Display for Proof<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl<P> error::Error for Proof<P> {}

#[macro_export]
macro_rules! prove {
    ($prove:expr) => {{
        let prove = $prove;
        if Prove::prove(&prove) {
            Ok(())
        } else {
            Err(Proof(stringify!($prove), prove))
        }
    }};
}
