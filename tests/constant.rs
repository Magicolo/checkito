#![cfg(feature = "constant")]

pub mod common;
use common::*;

#[test]
fn boba() {
    use crate as checkito;
    let a = constant!(1);
}
