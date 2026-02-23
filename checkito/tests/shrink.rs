pub mod common;
use common::*;

/// For each non-finite float value produced by the full generator, the first
/// shrink step must produce a finite probe value.
#[test]
fn float_non_finite_shrinker_has_finite_probe() {
    for mut s in shrinker(f32::generator()).samples(5_000) {
        let item = s.item();
        if !item.is_finite() {
            let probe = s.shrink();
            assert!(
                probe.is_some(),
                "non-finite value {item} should have a first shrink step"
            );
            assert!(
                probe.unwrap().item().is_finite(),
                "first shrink of non-finite {item} should be a finite value"
            );
        }
    }
    for mut s in shrinker(f64::generator()).samples(5_000) {
        let item = s.item();
        if !item.is_finite() {
            let probe = s.shrink();
            assert!(
                probe.is_some(),
                "non-finite value {item} should have a first shrink step"
            );
            assert!(
                probe.unwrap().item().is_finite(),
                "first shrink of non-finite {item} should be a finite value"
            );
        }
    }
}

/// When the full float generator produces INFINITY, the shrinker should first
/// probe f32::MAX / f64::MAX (the largest finite value).
#[test]
fn float_infinity_first_probe_is_max() {
    let found = shrinker(f32::generator())
        .samples(5_000)
        .filter(|s| s.item() == f32::INFINITY)
        .next();
    if let Some(mut s) = found {
        let probe = s.shrink().expect("INFINITY should shrink");
        assert_eq!(probe.item(), f32::MAX, "INFINITY should shrink to MAX first");
    }
    let found = shrinker(f64::generator())
        .samples(5_000)
        .filter(|s| s.item() == f64::INFINITY)
        .next();
    if let Some(mut s) = found {
        let probe = s.shrink().expect("INFINITY should shrink");
        assert_eq!(probe.item(), f64::MAX, "INFINITY should shrink to MAX first");
    }
}

/// When the full float generator produces NEG_INFINITY, the shrinker should
/// first probe f32::MIN / f64::MIN (the most negative finite value).
#[test]
fn float_neg_infinity_first_probe_is_min() {
    let found = shrinker(f32::generator())
        .samples(5_000)
        .filter(|s| s.item() == f32::NEG_INFINITY)
        .next();
    if let Some(mut s) = found {
        let probe = s.shrink().expect("NEG_INFINITY should shrink");
        assert_eq!(
            probe.item(),
            f32::MIN,
            "NEG_INFINITY should shrink to MIN first"
        );
    }
    let found = shrinker(f64::generator())
        .samples(5_000)
        .filter(|s| s.item() == f64::NEG_INFINITY)
        .next();
    if let Some(mut s) = found {
        let probe = s.shrink().expect("NEG_INFINITY should shrink");
        assert_eq!(
            probe.item(),
            f64::MIN,
            "NEG_INFINITY should shrink to MIN first"
        );
    }
}

/// When the full float generator produces NaN, the shrinker should first probe
/// 0.0 (the simplest finite value).
#[test]
fn float_nan_first_probe_is_zero() {
    let found = shrinker(f32::generator())
        .samples(5_000)
        .filter(|s| s.item().is_nan())
        .next();
    if let Some(mut s) = found {
        let probe = s.shrink().expect("NaN should shrink");
        assert_eq!(probe.item(), 0.0_f32, "NaN should shrink to 0.0 first");
    }
    let found = shrinker(f64::generator())
        .samples(5_000)
        .filter(|s| s.item().is_nan())
        .next();
    if let Some(mut s) = found {
        let probe = s.shrink().expect("NaN should shrink");
        assert_eq!(probe.item(), 0.0_f64, "NaN should shrink to 0.0 first");
    }
}

/// All shrinkers produced by the full float generator (including non-finite
/// values) must eventually converge to 0.0 after exhaustive shrinking.
#[test]
fn float_generator_shrinker_converges_to_zero() {
    for mut outer in shrinker(f32::generator()).samples(5_000) {
        while let Some(inner) = outer.shrink() {
            outer = inner;
        }
        assert_eq!(outer.item(), 0.0_f32);
    }
    for mut outer in shrinker(f64::generator()).samples(5_000) {
        while let Some(inner) = outer.shrink() {
            outer = inner;
        }
        assert_eq!(outer.item(), 0.0_f64);
    }
}

#[test]
fn finds_minimum() {
    let fail = <(usize, usize)>::generator()
        .check(|(left, right)| left >= right)
        .unwrap();
    assert_eq!(fail.item, (0, 1));
}

#[test]
fn integer_shrink_to_minimum() {
    for high in (1usize..1_000_000_000).samples(1_000) {
        let fail = usize::generator().check(|item| item < high).unwrap();
        assert_eq!(fail.item, high);
    }
}

#[test]
fn vec_removes_irrelevant_then_shrinks() {
    let fail = (..100usize)
        .collect::<Vec<_>>()
        .check(|items| items.len() < 10 || items.iter().all(|&item| item < 10))
        .unwrap();
    let shrunk = fail.item;
    assert_eq!(shrunk.len(), 10);
    assert_eq!(shrunk.iter().filter(|&&item| item == 10).count(), 1);
}
