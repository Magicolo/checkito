use std::{any::Any, error, fmt};

pub trait Prove: 'static {
    fn prove(&self) -> bool;
    fn is(&self, other: &dyn Any) -> bool;
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

impl Prove for bool {
    fn prove(&self) -> bool {
        *self
    }

    fn is(&self, other: &dyn Any) -> bool {
        other
            .downcast_ref::<Self>()
            .map_or(false, |other| self == other)
    }
}

impl<T: 'static, E: 'static> Prove for Result<T, E> {
    fn prove(&self) -> bool {
        self.is_ok()
    }

    fn is(&self, other: &dyn Any) -> bool {
        other
            .downcast_ref::<Self>()
            .map_or(false, |other| match (self, other) {
                (Ok(left), Ok(right)) => left.type_id() == right.type_id(),
                (Err(left), Err(right)) => {
                    match (
                        (left as &dyn Any).downcast_ref::<Error>(),
                        (right as &dyn Any).downcast_ref::<Error>(),
                    ) {
                        (Some(left), Some(right)) => {
                            left.line == right.line
                                && left.column == right.column
                                && left.file == right.file
                                && left.module == right.module
                                && left.expression == right.expression
                        }
                        _ => left.type_id() == right.type_id(),
                    }
                }
                _ => false,
            })
    }
}

#[macro_export]
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
