pub mod common;
use common::*;

#[test]
fn positive_towards_zero() {
    assert!((f64::EPSILON..f64::MAX)
        .check(|value| value > value.nudge(-1.0))
        .is_none());
}

#[test]
fn positive_towards_maximum() {
    assert!((f64::EPSILON..f64::MAX)
        .check(|value| value < value.nudge(1.0))
        .is_none());
}

#[test]
fn negative_towards_zero() {
    assert!((f64::MIN.nudge(-1.0)..=-f64::EPSILON)
        .check(|value| value < value.nudge(-1.0))
        .is_none());
}

#[test]
fn negative_towards_minimum() {
    assert!((f64::MIN.nudge(-1.0)..=-f64::EPSILON)
        .check(|value| value > value.nudge(1.0))
        .is_none());
}
