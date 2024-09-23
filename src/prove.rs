use core::{error, fmt};

pub trait Prove {
    fn prove(&self) -> bool;
}

#[derive(Clone, Debug)]
pub struct Error {
    pub value: String,
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

#[macro_export]
#[deprecated(since = "1.7.0", note = "use standard assertion macros instead")]
macro_rules! prove {
    ($prove:expr) => {{
        let prove = $prove;
        if $crate::prove::Prove::prove(&prove) {
            Ok(prove)
        } else {
            Err($crate::prove::Error {
                value: format!("{prove:?}"),
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
