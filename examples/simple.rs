use checkito::*;

fn main() -> Result<(), impl std::error::Error> {
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
            // - The `prove` macro is not strictly required but keeps some call site information if an error is encountered which can
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
                size: 0.0,
                seed: 6507210940052631922,
                random: Rng(Cell { value: 4301107024673349119 }),
            },
            original: (
                Input { value: 0, maximum: 0 },
                Err(result.is_some()),
            ),
            shrinks: Shrinks { accept: 0, reject: 0 },
            shrunk: None,
        };`
    */
    result
}