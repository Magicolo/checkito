# checkito

A simple [quickcheck](https://hackage.haskell.org/package/QuickCheck) inspired library to generate growable/shrinkable random data mainly oriented towards generative/property/exploratory testing. One would use this library to prove that certain properties hold for their programs for a tentatively representative sample of the input space.

-   The [`Generate`](src/generate.rs) trait that is implemented for many of rust's standard types allows the generation of any random composite data through combinator (such as tuples, `Any`, `Map`, `Flatten` and more). It is designed for composability and its usage should feel like working with `Iterator`s.
-   The [`Shrink`](src/shrink.rs) trait tries to reduce a generated sample to a 'smaller' version of it while maintaining its constraints (ex: a sample `usize` in the range `10..100` will never be shrunk out of its range). For numbers, it means bringing the sample closer to 0, for vectors, it means removing irrelevant items and shrinking the remaining ones, etc..
-   The [`Prove`](src/prove.rs) trait is meant to represent a desirable property of a system under test. It is used mainly in the context of the `Generate::check` or `Checker::check` methods and it is the failure of a proof that triggers the shrinking process. It is implemented for a couple of standard types such as `bool`, `Result` and tuples.


## Example

```rust
use checkito::*;

#[derive(Debug)]
struct Input {
    value: usize,
    maximum: usize,
}

fn filter_less_than(input: &Input) -> Option<usize> {
    if input.value <= input.maximum {
        Some(input.value)
    } else {
        None
    }
}

// - The type `usize` provides a generator for its full range using the `FullGenerate` implementation.
let value = usize::generator();
// - Ranges implement the `IntoGenerate` trait and will only generate values within their bounds.
let maximum = (0usize..1_000_000).generator();
// - Tuples implement `Generate` if their items also implement it.
let result = (value, maximum)
    // The `Generate::map` method combines the `usize` pair in the `Input` structure.
    .map(|(value, maximum)| Input { value, maximum })
    // The `Generate::check` method will generate 1000 `Input` values that will get gradually larger.
    .check(1000, |input| {
        let result = filter_less_than(input);
        // - The `prove` macro is not strictly required but it keeps some call site information if an error is encountered which can
        // simplify the debugging process. Any type that implements the `Prove` trait (including a simple `bool`) can be returned.
        // - This proof will fail for inputs where `input.value > input.maximum` and when this happens, `checkito` will
        // try to find the minimum sample that reproduces the failure.
        prove!(result.is_some())?;
        // Multiple proofs can be defined.
        prove!(result == Some(input.value))
    });

/*
    An error will hold the original value that triggered a failed proof and the smallest found shrinked version (and a bunch of additional information).
    A sample error may look like:
    `Error {
        state: State {
            size: 0.015400000000000002,
            seed: 3458476899729584474,
            random: Rng(Cell { value: 1252372984350301671 })
        },
        original: (
            Input { value: 3, maximum: 0, },
            Err(result.is_some()),
        ),
        shrinks: Shrinks { accept: 1, reject: 0, },
        shrunk: Some((
            Input { value: 1, maximum: 0, },
            Err(result.is_some()),
        )),
    }`
*/
```
