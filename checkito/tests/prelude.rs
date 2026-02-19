pub mod common;
use common::*;
use orn::{Or1, Or2, Or3, Or4};

pub fn is_generator<T>(_: impl Generate<Item = T>) {}

macro_rules! generators {
    ($type: ident, $value: expr, $or: ty, $($values: expr),+) => {
        mod $type {
            use super::*;

            #[test]
            fn prelude_generators_implement_generate() {
                is_generator::<$type>(same($value));
                is_generator::<Option<$type>>(any([$($values),*]));
                is_generator::<$or>(any(($($values,)*)));
                is_generator::<$type>(unify(any(($($values,)*))));
                is_generator::<$type>(map($value, |value| value));
                is_generator::<$type>(flat_map($value, same));
                is_generator::<$type>(flatten(same(same($value))));
                is_generator::<Option<$type>>(filter($value, |_| true, 1));
                is_generator::<Option<$type>>(filter_map($value, Some, 1));
                is_generator::<$type>(boxed(Box::new($value)));
                is_generator::<[$type; 1]>(array::<_, 1>($value));
                is_generator::<Vec<$type>>(collect($value, 1usize));
                is_generator::<$type>(size($value, |_| 1.0));
                is_generator::<$type>(dampen($value, 1.0, 1, 1));
                is_generator::<$type>(keep($value));
                is_generator::<Option<$type>>(convert($value));
                is_generator::<same::Same<$type>>(shrinker(same($value)));
                is_generator::<keep::Keep<$type>>(shrinker(keep($value)));
                is_generator::<convert::Convert<$type, $type>>(shrinker(convert($value)));
                is_generator::<$type>(with(|| $value));
                is_generator::<$type>(lazy(|| $value));
                is_generator::<$type>(cardinality::<_, 1>($value));
            }
        }
    };
}

generators!(u8, 1u8, Or1<u8>, 2u8);
generators!(i32, 1i32, Or2<i32, i32>, 2i32, 3i32);
generators!(char, 'a', Or3<char, char, char>, 'b', 'c', 'd');
generators!(bool, true, Or4<bool, bool, bool, bool>, false, true, false, false);

#[test]
fn size_can_force_minimal_collections() {
    let values = Generate::collect::<Vec<_>>(0u8..=u8::MAX)
        .size(|_| 0.0)
        .samples(64)
        .collect::<Vec<_>>();

    assert!(values.iter().all(Vec::is_empty));
}

#[test]
fn dampen_with_zero_limit_forces_minimal_collections() {
    let values = Generate::collect::<Vec<_>>(0u8..=u8::MAX)
        .dampen_with(1.0, 8, 0)
        .samples(64)
        .collect::<Vec<_>>();

    assert!(values.iter().all(Vec::is_empty));
}

#[test]
fn dampen_with_zero_deepest_forces_minimal_collections() {
    let values = Generate::collect::<Vec<_>>(0u8..=u8::MAX)
        .dampen_with(1.0, 0, usize::MAX)
        .samples(64)
        .collect::<Vec<_>>();

    assert!(values.iter().all(Vec::is_empty));
}

#[test]
fn dampen_with_limit_applies_after_nested_flatten_depth() {
    let values = same(same(
        Generate::collect::<Vec<_>>(0u8..=u8::MAX).dampen_with(1.0, usize::MAX, 1),
    ))
    .flatten()
    .flatten()
    .samples(32)
    .collect::<Vec<_>>();

    assert!(values.iter().all(Vec::is_empty));
}

#[test]
fn dampen_with_both_zero_forces_minimal_collections() {
    // When both deepest and limit are 0, size is always 0.0
    let values = Generate::collect::<Vec<_>>(0u8..=u8::MAX)
        .dampen_with(1.0, 0, 0)
        .samples(64)
        .collect::<Vec<_>>();

    assert!(values.iter().all(Vec::is_empty));
}

#[test]
fn dampen_with_high_depth_handles_gracefully() {
    // Test that very high depth values don't cause issues
    let values = Generate::collect::<Vec<_>>(0u8..=u8::MAX)
        .dampen_with(1.0, 50, usize::MAX)
        .samples(64)
        .collect::<Vec<_>>();

    // Should not panic and should generate reasonable collections
    // At depth 0, size should be normal, so some non-empty vectors expected
    assert!(values.iter().any(|v| !v.is_empty()));
}

#[test]
fn dampen_deepest_threshold_reached_first() {
    // When deepest is lower than limit, deepest threshold determines when size becomes 0
    // This test uses deepest=1, so after the first depth increase, size becomes 0
    let values = same(same(
        Generate::collect::<Vec<_>>(0u8..=u8::MAX).dampen_with(1.0, 1, 100),
    ))
    .flatten()
    .flatten()
    .samples(32)
    .collect::<Vec<_>>();

    // After reaching depth >= 1, size becomes 0.0, so all should be empty
    assert!(values.iter().all(Vec::is_empty));
}

#[test]
fn dampen_limit_threshold_reached_first() {
    // When limit is lower than deepest, limit threshold determines when size becomes 0
    let values = same(same(
        Generate::collect::<Vec<_>>(0u8..=u8::MAX).dampen_with(1.0, 100, 1),
    ))
    .flatten()
    .flatten()
    .samples(32)
    .collect::<Vec<_>>();

    // After limit >= 1, size becomes 0.0
    assert!(values.iter().all(Vec::is_empty));
}


#[test]
fn lazy_constructs_generator_only_once() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let calls = Arc::new(AtomicUsize::new(0));
    let lazy = {
        let calls = Arc::clone(&calls);
        lazy(move || {
            calls.fetch_add(1, Ordering::SeqCst);
            0u8..=1
        })
    };

    let samples = lazy.samples(128).collect::<Vec<_>>();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(samples.iter().all(|value| *value <= 1));
}

#[test]
fn standard_option_generator_covers_some_and_none() {
    let values = <Option<bool>>::generator().samples(512).collect::<Vec<_>>();

    assert!(values.iter().any(Option::is_none));
    assert!(values.iter().any(|value| value == &Some(false)));
    assert!(values.iter().any(|value| value == &Some(true)));
}

#[test]
fn standard_result_generator_covers_ok_and_err() {
    let values = <Result<bool, bool>>::generator()
        .samples(512)
        .collect::<Vec<_>>();

    assert!(values.iter().any(|value| value == &Ok(false)));
    assert!(values.iter().any(|value| value == &Ok(true)));
    assert!(values.iter().any(|value| value == &Err(false)));
    assert!(values.iter().any(|value| value == &Err(true)));
}

#[test]
fn lazy_cell_generator_is_forced_and_reused() {
    use core::cell::LazyCell;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let calls = Arc::new(AtomicUsize::new(0));
    let generator = {
        let calls = Arc::clone(&calls);
        LazyCell::new(move || {
            calls.fetch_add(1, Ordering::SeqCst);
            0u8..=2
        })
    };

    let values = generator.samples(64).collect::<Vec<_>>();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(values.iter().all(|value| *value <= 2));
}

#[test]
fn lazy_lock_generator_is_forced_and_reused() {
    use std::sync::{
        Arc, LazyLock,
        atomic::{AtomicUsize, Ordering},
    };

    let calls = Arc::new(AtomicUsize::new(0));
    let generator = {
        let calls = Arc::clone(&calls);
        LazyLock::new(move || {
            calls.fetch_add(1, Ordering::SeqCst);
            0u8..=2
        })
    };

    let values = generator.samples(64).collect::<Vec<_>>();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(values.iter().all(|value| *value <= 2));
}

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check(0u8..=u8::MAX)]
    fn converted_values_match_from_implementation(value: u8) {
        let converted = same(value).convert::<u16>().samples(1).next().unwrap();
        assert_eq!(converted, u16::from(value));
    }
}
