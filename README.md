# checkito

A simple and efficient [quickcheck](https://hackage.haskell.org/package/QuickCheck) inspired library to generate random data mainly oriented towards generative/property/exploratory testing. One would use this library to prove that certain properties hold for their programs for a tentatively representative sample of the input space.

-   The `Generate` trait that is implemented for many of rust's standard types allows the generation of any random composite data through combinator (such as tuples, `Any`, `Map`, `Flatten` and more).
-   The `Prove` trait is meant to represent a desirable property of a program. It is implemented for a couple of standard types such as `bool` and `Result`.
-   When a generated sample is shown to disprove a desired property, the `Shrink` trait tries to iteratively 'reduce' it to a more minimal form.

_More information to come..._
