#![doc = include_str!("../../README.md")]
// #![forbid(unsafe_code)]

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
#[cfg(feature = "parallel")]
mod parallel;
mod prelude;
pub mod primitive;
pub mod prove;
pub mod regex;
#[doc(hidden)]
pub mod run;
pub mod same;
pub mod sample;
pub mod shrink;
pub mod size;
pub mod standard;
pub mod state;
pub mod unify;
mod utility;

pub use check::Check;
/// Turns a function into a property test.
///
/// It takes a list of generators as input, which are used to produce random
/// arguments for the function it's attached to.
///
/// The function can return `()`, `bool`, or `Result` to indicate whether the
/// property holds. Any `panic` within the function is also treated as a test
/// failure.
///
/// # Arguments
///
/// - **Generators**: The first arguments to the macro are a comma-separated
///   list of generators. The values produced by these generators will be passed
///   as arguments to the test function. You can use `_` to infer the default
///   generator for a type.
/// - `verbose`: A boolean (`true` or `false`) to enable or disable verbose
///   output, which shows every generation and shrink step. Defaults to `false`.
/// - `color`: A boolean (`true` or `false`) to enable or disable colored
///   output. Defaults to `true`.
/// - `debug`: A boolean (`true` or `false`) that controls the output format. If
///   `true`, the full `Debug` representation of test results is printed. If
///   `false`, a more minimal output is used. Defaults to `true`.
///
/// # Examples
///
/// A simple test with a range generator:
/// ```
/// # use checkito::check;
/// #[check(0..100)]
/// fn is_less_than_100(x: i32) {
///     assert!(x < 100);
/// }
/// ```
///
/// Using multiple generators and inferring a type:
/// ```
/// # use checkito::check;
/// #[check(.., 0.0..1.0, _)]
/// fn complex_test(x: i32, y: f64, z: bool) {
///     // ...
/// }
/// ```
///
/// Disabling color and enabling verbose output:
/// ```
/// # use checkito::check;
/// #[check(0..10, color = false, verbose = true)]
/// #[should_panic]
/// fn failing_test(x: i32) {
///     assert!(x > 5);
/// }
/// ```
#[cfg(feature = "check")]
pub use checkito_macro::check;
/// Creates a generator from a compile-time constant value.
///
/// This macro is useful for embedding constant values directly into generators.
/// This is mainly relevant for static cardinality estimates. When possible, the
/// expression inside the macro is converted to a generator with constant
/// parameters.
#[cfg(feature = "constant")]
pub use checkito_macro::constant;
/// Creates a generator from a regular expression, validated at compile time.
///
/// This macro takes a string literal representing a regular expression and
/// produces a generator that yields strings matching that pattern. The regex is
/// parsed and validated at compile time, so any errors in the pattern will
/// result in a compilation failure.
///
/// # Examples
///
/// ```
/// # use checkito::{check, regex};
/// #[check(regex!("[a-zA-Z0-9]{1,10}"))]
/// fn has_alphanumeric_content(s: String) {
///     assert!(s.chars().all(|c| c.is_alphanumeric()));
///     assert!(s.len() >= 1 && s.len() <= 10);
/// }
/// ```
#[cfg(feature = "regex")]
pub use checkito_macro::regex;
pub use generate::{FullGenerate, Generate};
pub use prelude::*;
pub use prove::Prove;
pub use sample::Sample;
pub use shrink::Shrink;

const GENERATES: usize = 1 << 10;
const SHRINKS: usize = 1 << 20;
const SAMPLES: usize = 1 << 7;
const COLLECTS: usize = 1 << 10;
const RETRIES: usize = 1 << 8;
#[cfg(feature = "regex")]
const REPEATS: u32 = 1 << 6;

/*
    TODO:
    - Asynchronous checks seem to hang forever. Add tests.
    - Instead of running a fixed number of checks, determine the number of checks based on the runtime of the generation and check.
    - Support for 'async' checks.
        - The check attribute can automatically detect this based on the 'async' keyword of the function.
    - Support for 'parallel' checks.
    - Review public api and make things more private to prevent breaking changes; especially modules.
    - Remove this list from release.
*/
