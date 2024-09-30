use checkito::*;

fn main() {
    #[derive(Debug)]
    struct Input {
        value: usize,
        maximum: usize,
    }

    fn filter_less_than(input: &Input) -> Option<usize> {
        if input.value <= 1000 || input.value <= input.maximum {
            Some(input.value)
        } else {
            None
        }
    }

    // - The type `usize` provides a generator for its full range using the `FullGenerate` implementation.
    let value = usize::generator();
    // - Ranges implement the `IntoGenerate` trait and will only generate values within their bounds.
    let maximum = (..1_000_000usize).generator();
    // - Tuples implement `Generate` if their items also implement it.
    let result = (value, maximum)
        // The `Generate::map` method combines the `usize` pair in the `Input` structure.
        .map(|(value, maximum)| Input { value, maximum })
        // The `Generate::check` method will generate 1000 `Input` values that will get gradually larger.
        .check(|input| {
            let result = filter_less_than(&input);
            // - This assertion will fail for inputs where `input.value > 1000 && input.value > input.maximum` and when this happens,
            // `checkito` will try to find the minimum sample that reproduces the failure.
            assert!(result.is_some());
            // Assertions can also be used.
            assert!(input.maximum < 1_000_000);
            // Multiple proofs can be defined.
            assert_eq!(result, Some(input.value));
        });

    dbg!(result.unwrap_err());
    /*
        An error will hold the original value that triggered a failed proof and the smallest found shrinked version (and a bunch of additional information).
        A sample error may look like:
        `Error {
            state: State { .. },
            cause: Disprove(Err(Error {
                prove: false,
                expression: "result.is_some()",
                file: "examples\\simple.rs",
                module: "simple",
                line: 33,
                column: 13,
            })),
            original: Input { value: 1663, maximum: 257, },
            shrunk: Some(Input { value: 1001, maximum: 0, }),
            shrinks: Shrinks { .. },
        }`
    */
}
