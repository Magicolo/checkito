pub mod common;
use checkito::any::{Unify, Weight};
use common::*;

#[test]
fn weighted_any() {
    let samples = (
        Weight::new(1.0, 1),
        Weight::new(10.0, 10),
        Weight::new(100.0, 100),
    )
        .map(Unify::<i32>::unify)
        .samples(1000)
        .collect::<Vec<_>>();
    let _1 = samples.iter().filter(|&&value| value == 1).count();
    let _10 = samples.iter().filter(|&&value| value == 10).count();
    let _100 = samples.iter().filter(|&&value| value == 100).count();
    assert!(_1 < _10);
    assert!(_10 < _100);
}
