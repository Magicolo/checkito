# checkito

### A simple [quickcheck](https://hackage.haskell.org/package/QuickCheck) inspired library to generate growable/shrinkable random data mainly oriented towards generative/property/exploratory testing. One would use this library to prove that certain properties hold for their programs for a tentatively representative sample of the input space.

-   The [`Generate`](src/generate.rs) trait that is implemented for many of rust's standard types allows the generation of any random composite data through combinator (such as tuples, `Any`, `Map`, `Flatten` and more). It is designed for composability and its usage should feel like working with `Iterator`s.
-   The [`Shrink`](src/shrink.rs) trait tries to reduce a generated sample to a 'smaller' version of it while maintaining its constraints (ex: a sample `usize` in the range `10..100` will never be shrunk out of its range). For numbers, it means bringing the sample closer to 0, for vectors, it means removing irrelevant items and shrinking the remaining ones, etc..
-   The [`Prove`](src/prove.rs) trait is meant to represent a desirable property of a system under test. It is used mainly in the context of the `Generate::check` or `Checker::check` methods and it is the failure of a proof that triggers the shrinking process. It is implemented for a couple of standard types such as `bool`, `Result` and tuples.


## Example

```rust
use checkito::{check::Error, regex::Regex, *};

// Parse this pattern as a `Regex` which implements the `Generate` trait.
let regex = "[a-zA-Z0-9_]*".parse::<Regex>().unwrap();
// Generate 1000 `String` values which are checked to be alphanumeric.
// `Generate::check` will fail when a '_' will appear in the value and the shrinking process will begin.
let result: Result<_, _> = regex.check(1000, |value: &String| value.chars().all(|character| character.is_alphanumeric()));
// `result` will be `Err` and will hold the original and shrunk values.
let error: Error<String, _> = result.unwrap_err();
let original: &String = error.original();
let shrunk: &String = error.shrunk();

// Alternatively, generated samples can be retrieved directly, bypassing shrinking.
for value in regex.samples(1000) {
    assert!(value.chars().all(|character| character.is_alphanumeric()));
}
```

_See the [examples](examples/) folder for more detailed examples._