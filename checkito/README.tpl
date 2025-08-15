<div align="center"> <h1> {{ package.name }} {{ package.version }} </h1> </div>

<p align="center">
    <em> 
{{ package.description }}
    </em>
</p>

<div align="right">
    <a href="https://github.com/Magicolo/{{ package.name }}/actions/workflows/test.yml"> <img src="https://github.com/Magicolo/{{ package.name }}/actions/workflows/test.yml/badge.svg"> </a>
    <a href="https://crates.io/crates/{{ package.name }}"> <img src="https://img.shields.io/crates/v/{{ package.name }}.svg"> </a>
</div>

---
### In Brief

The purpose of the library is to test general properties of a program rather than very specific examples as you would with unit tests. 

- When writing a `{{ package.name }}` test (called a `check`), you first construct a generator by specifying the bounds that make sense for the inputs (ex: a number in the range `10..100`, an alpha-numeric string, a vector of `f64`, etc.). 
- Generators can produce arbitrary complex values with their combinators, in a similar way that `Iterator`s can.
- Given a proper generator, `{{ package.name }}` will sample the input space to find a failing case for your test.
- Once a failing case is found, `{{ package.name }}` will try to reduce the input to the simplest version of it that continues to fail (using a kind of binary search of the input space) to make the debugging process easier.
- Note that `{{ package.name }}` does not guarantee any kind of exhaustive search of the input space (the size of it gets out of hand rather quickly) and is meant as a complement to other testing strategies.
- It is recommended to write a regular unit test with the exact failing input to prevent a regression and to truly guarantee that the failing input is always tested.

---
### Main Concepts

The library is built around a few core traits:

-   [`Generate`](src/generate.rs): is implemented for many of rust's standard types and allows the generation of any random composite/structured data through combinator (such as tuples, [`Any`](src/any.rs), [`Map`](src/map.rs), [`Flatten`](src/flatten.rs) and more). It is designed for composability and its usage should feel like working with `Iterator`s.
-   [`Shrink`](src/shrink.rs): tries to reduce a generated sample to a 'smaller' version of it while maintaining its constraints (ex: a sample `usize` in the range `10..100` will never be shrunk below `10`). For numbers, it means bringing the sample closer to 0, for vectors, it means removing irrelevant items and shrinking the remaining ones, and so on.
-   [`Prove`](src/prove.rs): represents a desirable property of a program under test. It is used mainly in the context of the [`Check::check`](src/check.rs) or [`Checker::check`](src/check.rs) methods and it is the failure of a proof that triggers the shrinking process. It is implemented for a couple of standard types such as `()`, `bool` and `Result`. A `panic!()` is also considered as a failing property, thus standard `assert!()` macros (or any other panicking assertions) can be used to check the property.
-   [`Check`](src/check.rs): A trait (implemented for all `Generate` types) that provides the main entry points for running property tests: `check` and `checks`.
   
*To ensure safety, this library has `#![forbid(unsafe_code)]`.*

---
### Environment Variables

The behavior of the test runner can be configured through environment variables, which is
particularly useful for CI environments or for debugging specific issues.

#### Generation
- `CHECKITO_GENERATE_COUNT`: Overrides the number of test cases to run.
  Example: `CHECKITO_GENERATE_COUNT=1000 cargo test`
- `CHECKITO_GENERATE_SIZE`: Sets a fixed generation size (`0.0` to `1.0`).
  Example: `CHECKITO_GENERATE_SIZE=1.0 cargo test`
- `CHECKITO_GENERATE_SEED`: Sets the initial seed for the random number generator, allowing
  for reproducible test runs.
- `CHECKITO_GENERATE_ITEMS`: Sets whether to display passing generation items (`true` or `false`).

#### Shrinking
- `CHECKITO_SHRINK_COUNT`: Overrides the maximum number of shrink attempts.
- `CHECKITO_SHRINK_ITEMS`: Sets whether to display passing shrink items (`true` or `false`).
- `CHECKITO_SHRINK_ERRORS`: Sets whether to display failing shrink items (`true` or `false`).

---
### Cheat Sheet

```rust
{% include "cheat.rs" %}
```

_See the [examples](examples/) and [tests](tests/) folder for more detailed examples._

---
### Contribute
- If you find a bug or have a feature request, please open an [issues](https://github.com/Magicolo/{{ package.name }}/issues).
- `{{ package.name }}` is actively maintained and [pull requests](https://github.com/Magicolo/{{ package.name }}/pulls) are welcome.
- If `{{ package.name }}` was useful to you, please consider leaving a [star](https://github.com/Magicolo/{{ package.name }})!

---
### Alternatives
- [proptest](https://crates.io/crates/proptest)
- [quickcheck](https://crates.io/crates/quickcheck)
- [arbitrary](https://crates.io/crates/arbitrary)
- [monkey_test](https://crates.io/crates/monkey_test)