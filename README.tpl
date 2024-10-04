<div align="center"> <h1> {{name}} {{version}} </h1> </div>

<p align="center">
    <em> 

{{description}}
    </em>
</p>

<div align="right">
    <a href="https://github.com/Magicolo/{{name}}/actions/workflows/test.yml"> <img src="https://github.com/Magicolo/{{name}}/actions/workflows/test.yml/badge.svg"> </a>
    <a href="https://crates.io/crates/{{name}}"> <img src="https://img.shields.io/crates/v/{{name}}.svg"> </a>
</div>

---
### In Brief

The purpose of the library is to test general properties of a program rather than very specific examples as you would with unit tests. 

- When writing a `{{name}}` test (called a `check`), you first construct a generator by specifying the bounds that make sense for the inputs (ex: a number in the range `10..100`, an alpha-numeric string, a vector of `f64`, etc.). 
- Generators can produce arbitrary complex values with its combinators in a similar way that `Iterator`s can.
- Given a proper generator, `{{name}}` will sample the input space to find a failing case for your test.
- Once a failing case is found, `{{name}}` will try to reduce the input to the simplest version of it that continues to fail (using a kind of binary search of the input space) to make the debugging process much easier.
- Note that `{{name}}` does not guarantee any kind of exhaustive search of the input space (the size of it gets out of hand rather quickly) and is meant as a complement to other testing strategies.
- It is recommended to write a regular unit test with the exact failing input to prevent a regression and to truly guarantee that the failing input is always tested.

---
### Main Traits
-   [`Generator`](src/generate.rs): is implemented for many of rust's standard types and allows the generation of any random composite/structured data through combinator (such as tuples, [`Any`](src/any.rs), [`Map`](src/map.rs), [`Flatten`](src/flatten.rs) and more). It is designed for composability and its usage should feel like working with `Iterator`s.
-   [`Shrinker`](src/shrink.rs): tries to reduce a generated sample to a 'smaller' version of it while maintaining its constraints (ex: a sample `usize` in the range `10..100` will never be shrunk below `10`). For numbers, it means bringing the sample closer to 0, for vectors, it means removing irrelevant items and shrinking the remaining ones, and so on.
-   [`Prove`](src/prove.rs): represents a desirable property of a program under test. It is used mainly in the context of the [`Check::check`](src/check.rs) or [`Checker::check`](src/check.rs) methods and it is the failure of a proof that triggers the shrinking process. It is implemented for a couple of standard types such as `()`, `bool` and `Result`. A `panic!()` is also considered as a failing property, thus standard `assert!()` macros (or any other panicking assertions) can be used to check the property.
   
*To ensure safety, this library is `#![forbid(unsafe_code)]`.*

---
### Cheat Sheet

```rust
{{examples/cheat.rs}}
```

_See the [examples](examples/) and [tests](tests/) folder for more detailed examples._

---
### Contribute
- If you find a bug or have a feature request, please open an [issues](https://github.com/Magicolo/{{name}}/issues).
- `{{name}}` is actively maintained and [pull requests](https://github.com/Magicolo/{{name}}/pulls) are welcome.
- If `{{name}}` was useful to you, please consider leaving a [star](https://github.com/Magicolo/{{name}})!

---
### Alternatives
- [proptest](https://crates.io/crates/proptest)
- [quickcheck](https://crates.io/crates/quickcheck)
- [arbitrary](https://crates.io/crates/arbitrary)
- [monkey_test](https://crates.io/crates/monkey_test)