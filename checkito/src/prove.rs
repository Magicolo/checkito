use core::convert::Infallible;

/// A trait that represents a property being tested.
///
/// The `Prove` trait is the mechanism by which `checkito` determines whether a
/// property test has passed or failed. The test function in a [`macro@crate::check`] macro
/// must return a type that implements this trait.
///
/// The outcome of the test is determined by the `Result` returned by the [`Prove::prove`]
/// method. An `Ok` variant signifies a pass, while an `Err` variant signifies a
/// failure. Any `panic` within the test function is also treated as a failure.
///
/// # Provided Implementations
///
/// `checkito` provides implementations for common return types:
///
/// - **`()`**: A function that returns unit `()` will always pass (unless it panics).
///   This is useful for tests that use `assert!` macros for their checks.
/// - **`bool`**: A function that returns `true` passes, and one that returns `false` fails.
/// - **`Result<T, E>`**: A function that returns `Ok(T)` passes, and one that returns
///   `Err(E)` fails. The success and error types can be anything.
///
/// # Examples
///
/// Using `()` with `assert!`:
/// ```
/// # use checkito::check;
/// #[check(0..100)]
/// fn test_with_assert(x: i32) {
///     assert!(x < 100); // This function implicitly returns `()`
/// }
/// ```
///
/// Using `bool`:
/// ```
/// # use checkito::check;
/// #[check(0..100)]
/// fn test_with_bool(x: i32) -> bool {
///     x < 100
/// }
/// ```
///
/// Using `Result`:
/// ```
/// # use checkito::check;
/// #[check(0..100)]
/// fn test_with_result(x: i32) -> Result<(), &'static str> {
///     if x < 100 {
///         Ok(())
///     } else {
///         Err("x was not less than 100")
///     }
/// }
/// ```
pub trait Prove {
    /// The type produced when the property holds (the test passes).
    type Proof;
    /// The type produced when the property is violated (the test fails).
    type Error;
    /// Evaluates the property, returning `Ok` for a pass and `Err` for a fail.
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
