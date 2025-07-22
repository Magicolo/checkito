#![cfg(feature = "asynchronous")]

pub mod common;
use common::*;
use core::future::{Future, ready};
use futures_lite::future::block_on;

#[test]
fn executes_to_completion() {
    block_on(
        bool::generator()
            .checker()
            .asynchronous()
            .check(|value| async move { value }),
    );
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
