#![cfg(feature = "parallel")]

pub mod common;
use checkito::check::Result;
use common::*;
use rayon::prelude::*;

#[test]
fn executes_to_completion() {
    for _ in 0..1_000_000 {
        let results = usize::generator()
            .checker()
            .parallel()
            .checks(|value| value < 1_000_000)
            .collect::<Vec<_>>();
        // assert!(matches!(results.first().unwrap(), Result::Pass(..)));
        assert!(matches!(results.last().unwrap(), Result::Fail(..)));
        // assert_eq!(value.item, 1_000_000);
    }
}

// #[cfg(feature = "check")]
// mod check {
//     use super::*;

//     #[check]
//     async fn compiles_with_async_function() {}

//     #[check(asynchronous = true)]
//     fn compiles_with_asynchronous_option() -> impl Future<Output = ()> {
//         ready(())
//     }
// }
