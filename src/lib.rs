#![forbid(unsafe_code)]

pub mod all;
pub mod any;
pub mod array;
pub mod boxed;
pub mod check;
pub mod collect;
pub mod convert;
pub mod dampen;
pub mod filter;
pub mod filter_map;
pub mod flatten;
pub mod fuse;
pub mod generate;
pub mod keep;
pub mod map;
pub mod nudge;
mod prelude;
pub mod primitive;
pub mod prove;
pub mod random;
pub mod regex;
pub mod same;
pub mod sample;
pub mod shrink;
pub mod size;
pub mod standard;
mod utility;

pub use check::Check;
#[cfg(feature = "check")]
pub use checkito_macro::check;
#[cfg(feature = "regex")]
pub use checkito_macro::regex;
pub use generate::{FullGenerator, Generator};
pub use prelude::*;
pub use prove::Prove;
#[cfg(feature = "regex")]
pub use regex::Regex;
pub use same::Same;
pub use sample::Sample;
pub use shrink::Shrinker;
