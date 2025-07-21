#![cfg(all(feature = "check", feature = "asynchronous"))]

pub mod common;
use async_io::block_on;
use common::*;
use core::future::{Future, ready};

#[test]
fn executes_to_completion() {
    block_on(
        bool::generator()
            .checker()
            .asynchronous()
            .check(|value| async move { value }),
    );
}

#[check]
async fn compiles_with_async_function() {}

#[check(asynchronous = true)]
fn compiles_with_asynchronous_option() -> impl Future<Output = ()> {
    ready(())
}
