use std::{error, fmt};

pub trait Prove {
    fn prove(&self) -> bool;
}

#[derive(Clone, Copy)]
pub struct Proof<P> {
    pub name: &'static str,
    pub prove: P,
}

impl<P> fmt::Debug for Proof<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl<P> fmt::Display for Proof<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.name, f)
    }
}

impl<P> error::Error for Proof<P> {}

#[macro_export]
macro_rules! prove {
    ($prove:expr) => {{
        let prove = $prove;
        if $crate::prove::Prove::prove(&prove) {
            Ok(())
        } else {
            Err($crate::prove::Proof {
                name: stringify!($prove),
                prove,
            })
        }
    }};
}
