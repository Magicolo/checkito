#![forbid(unsafe_code)]

pub mod all;
pub mod any;
pub mod array;
pub mod boxed;
pub mod cardinality;
pub mod check;
pub mod collect;
pub mod convert;
pub mod dampen;
pub mod filter;
pub mod filter_map;
pub mod flatten;
pub mod generate;
pub mod keep;
pub mod lazy;
pub mod map;
mod prelude;
pub mod primitive;
pub mod prove;
pub mod regex;
pub mod same;
pub mod sample;
pub mod shrink;
pub mod size;
pub mod standard;
pub mod state;
pub mod unify;
mod utility;

pub use check::Check;
#[cfg(feature = "check")]
pub use checkito_macro::check;
#[cfg(feature = "regex")]
pub use checkito_macro::regex;
pub use generate::{FullGenerate, Generate};
pub use prelude::*;
pub use prove::Prove;
pub use sample::Sample;
pub use shrink::Shrink;

const COLLECT: usize = 1024;
const RETRIES: usize = 256;
#[cfg(feature = "regex")]
const REPEATS: u32 = 64;

/*
    TODO:
    - Create a U8<const N: u8>; type for each primitive such that a range between those
    constants can have a precise static cardinality.
*/
