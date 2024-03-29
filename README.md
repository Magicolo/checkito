<div align="center"> <h1> checkito </h1> </div>

<p align="center">
    <em> A simple <a href="https://hackage.haskell.org/package/QuickCheck">QuickCheck</a> inspired library to generate shrinkable random data mainly oriented towards generative/property/exploratory testing. One would use this library to prove that certain properties hold for a program for a tentatively representative sample of their input space. </em>
</p>

<div align="right">
    <a href="https://github.com/Magicolo/checkito/actions/workflows/test.yml"> <img src="https://github.com/Magicolo/checkito/actions/workflows/test.yml/badge.svg"> </a>
    <a href="https://crates.io/crates/checkito"> <img src="https://img.shields.io/crates/v/checkito.svg"> </a>
</div>
<p/>

---

-   The [`Generate`](src/generate.rs) trait that is implemented for many of rust's standard types allows the generation of any random composite data through combinator (such as tuples, [`Any`](src/any.rs), [`Map`](src/map.rs), [`Flatten`](src/flatten.rs) and more). It is designed for composability and its usage should feel like working with `Iterator`s.
-   The [`Shrink`](src/shrink.rs) trait tries to reduce a generated sample to a 'smaller' version of it while maintaining its constraints (ex: a sample `usize` in the range `10..100` will never be shrunk out of its range). For numbers, it means bringing the sample closer to 0, for vectors, it means removing irrelevant items and shrinking the remaining ones, etc..
-   The [`Prove`](src/prove.rs) trait is meant to represent a desirable property of a system under test. It is used mainly in the context of the [`Generate::check`](src/generate.rs) or [`Checker::check`](src/check.rs) methods and it is the failure of a proof that triggers the shrinking process. It is implemented for a couple of standard types such as `bool` and `Result`.


## Example

```rust, should_panic
use checkito::{check::Error, *};

struct Composite(String, f64);

fn main() {
    // Parse this pattern as a [`Regex`] which implements the [`Generate`] trait. The '_' character is included in the regex
    // to make the checks below fail (for illustration purposes).
    let regex = regex!("[a-zA-Z0-9_]*");
    // [`f64`] ranges implement the [`Generate`] trait.
    let number = 10.0f64..;
    // Combine the previous [`Generate`] implementations and map them to a custom `struct`.
    let composite = (regex, number).map(|pair| Composite(pair.0, pair.1));

    // Generate 1000 [`Composite`] values which are checked to be alphanumeric.
    // [`Generate::check`] will fail when a '_' will appear in `value.0` and the shrinking process will begin.
    let result: Result<_, _> = composite.check(1000, |value: &Composite| {
        value.0.chars().all(|character| character.is_alphanumeric())
    });
    // `result` will be [`Err`] and will hold the original and shrunk values.
    let error: Error<Composite, _> = result.unwrap_err();
    let _original: &Composite = &error.original;
    // The expected shrunk value is [`Composite("_", 10.0)`].
    let _shrunk: &Option<Composite> = &error.shrunk;

    // Alternatively, generated samples can be retrieved directly, bypassing shrinking.
    for value in composite.samples(1000) {
        // This assertion is almost guaranteed to fail because of '_'.
        assert!(value.0.chars().all(|character| character.is_alphanumeric()));
    }
}
```

_See the [examples](examples/) and [tests](tests/) folder for more detailed examples._

## Alternatives
- [proptest](https://crates.io/crates/proptest)
- [quickcheck](https://crates.io/crates/quickcheck)
