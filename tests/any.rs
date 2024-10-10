pub mod common;
use checkito::any::Weight;
use common::*;

#[test]
fn weighted_any() {
    let samples = (
        Weight::new(1.0, 1),
        Weight::new(10.0, 10),
        Weight::new(100.0, 100),
    )
        .fuse::<i32>()
        .samples(1000)
        .collect::<Vec<_>>();
    let one = samples.iter().filter(|&&value| value == 1).count();
    let ten = samples.iter().filter(|&&value| value == 10).count();
    let hundred = samples.iter().filter(|&&value| value == 100).count();
    assert!(one < ten);
    assert!(ten < hundred);
}
