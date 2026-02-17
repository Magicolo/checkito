#![cfg(feature = "asynchronous")]

pub mod common;
use common::*;
use core::future::{Future, ready};
use futures_lite::future::block_on;

#[test]
fn executes_to_completion() {
    let fail = block_on(
        usize::generator()
            .checker()
            .asynchronous()
            .check(|value| async move { value < 1_000_000 }),
    );
    assert!(fail.is_some());
}

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check]
    async fn compiles_with_async_function() {}

    #[check(asynchronous = true)]
    fn compiles_with_asynchronous_option() -> impl Future<Output = ()> {
        ready(())
    }
}
