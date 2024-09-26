use core::{error, fmt};

pub trait Prove {
    fn prove(&self) -> bool;
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
    fn prove(&self) -> bool {
        true
    }
}

impl Prove for bool {
    fn prove(&self) -> bool {
        *self
    }
}

impl<T, E> Prove for Result<T, E> {
    fn prove(&self) -> bool {
        self.is_ok()
    }
}

impl<P: Prove> Prove for Option<P> {
    fn prove(&self) -> bool {
        match self {
            Some(prove) => prove.prove(),
            None => true,
        }
    }
}

impl Prove for Error {
    fn prove(&self) -> bool {
        false
    }
}

#[macro_export]
macro_rules! prove {
    ([$($values: expr),*] $prove:expr) => {{
        let prove = $prove;
        if $crate::prove::Prove::prove(&prove) {
            Ok(prove)
        } else {
            Err($crate::prove::Error {
                values: vec![$(format!("{:?}", $value)),*],
                expression: stringify!($prove),
                file: file!(),
                line: line!(),
                column: column!(),
                module: module_path!(),
            })
        }
    }};
    ($prove:expr) => {
        let prove = $prove;
        $crate::prove!([prove] prove);
    };
    ($($prove:expr),*) => { Ok(($($crate::prove!($prove)),*)) }
}
