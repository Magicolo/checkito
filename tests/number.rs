pub mod common;
use checkito::shrink::Shrinkers;
use common::*;

mod range {
    use super::*;

    macro_rules! tests {
        ($t:ident) => {
            mod $t {
                use super::*;

                #[test]
                fn has_sample() {
                    for i in 1..100 {
                        <$t>::generator().samples(i).next().unwrap();
                    }
                }

                #[test]
                fn sample_has_count() {
                    for i in 0..100 {
                        assert_eq!(<$t>::generator().samples(i).len(), i);
                    }
                }

                #[test]
                fn empty_range() {
                    assert!(number::<$t>().flat_map(|value| value..value).check(|_| true).is_none());
                }

                #[test]
                fn is_same() {
                    assert!(number::<$t>()
                        .flat_map(|value| (value, same(value)))
                        .check(|(left, right)| left == right)
                        .is_none());
                }

                #[test]
                fn has_extremes() {
                    let samples = $t::generator().samples(1000).collect::<Vec<_>>();
                    assert!(samples.contains(&$t::MIN));
                    assert!(samples.contains(&$t::MAX));
                    assert!(samples.contains(&(0 as $t)));
                }

                #[test]
                fn is_same_range() {
                    assert!(number::<$t>()
                        .flat_map(|value| (value, value..=value))
                        .check(|(left, right)| assert_eq!(left, right))
                        .is_none());
                }

                #[test]
                fn is_in_range() {
                    assert!((number::<$t>(), number::<$t>())
                        .map(|(low, high)| (low.min($t::MAX - $t::MAX / 100 as $t), high.min($t::MAX - $t::MAX / 100 as $t)))
                        .map(|(low, high)| (low.min(high), low.max(high) + $t::MAX / 100 as $t))
                        .flat_map(|(low, high)| (low..high, low, high))
                        .check(|(value, low, high)| value >= low && value < high)
                        .is_none());
                }

                #[test]
                fn is_in_range_inclusive() {
                    assert!((number::<$t>(), number::<$t>())
                        .map(|(low, high)| (low.min(high), low.max(high)))
                        .flat_map(|(low, high)| (low..=high, low, high))
                        .check(|(value, low, high)| value >= low && value <= high)
                        .is_none());
                }

                #[test]
                fn is_in_range_from() {
                    assert!(number::<$t>().flat_map(|low| (low, low..)).check(|(low, high)| low <= high)
                    .is_none());
                }

                #[test]
                fn is_in_range_to() {
                    assert!(number::<$t>()
                        .map(|high| high.max($t::MIN + $t::MAX / 100 as $t))
                        .flat_map(|high| (..high, high))
                        .check(|(low, high)| low < high)
                        .is_none());
                }

                #[test]
                fn is_in_range_to_inclusive() {
                    assert!(number::<$t>()
                        .flat_map(|high| (..=high, high))
                        .check(|(low, high)| low <= high)
                        .ok_or(true)
                        .unwrap_err());
                }

                #[test]
                fn is_positive() {
                    assert!(positive::<$t>()
                        .check(|value| value >= 0 as $t)
                        .is_none());
                }

                #[test]
                fn keeps_value() {
                    let fail = number::<$t>().keep().check(|value| value < 100 as $t).unwrap();
                    assert_eq!(fail.shrinks, 0);
                }

                #[test]
                fn shrinks_to_zero() {
                    for mut outer in Shrinkers::from(&number::<$t>()) {
                        while let Some(inner) = outer.shrink() {
                            outer = inner;
                        }
                        assert_eq!(0 as $t, outer.item());
                    }
                }

                #[test]
                fn shrinks_to_low_or_high() {
                    assert!(number::<$t>()
                        .flat_map(|value| {
                            if value < 0 as $t {
                                (value..=value, value..=0 as $t)
                            } else {
                                (0 as $t..=value, value..=value)
                            }
                        })
                        .flat_map(|(low, high)| (low, high, shrinker(low..=high)))
                        .check(|(low, high, mut outer)| {
                            while let Some(inner) = outer.shrink() {
                                outer = inner;
                            }
                            if low >= 0 as $t {
                                assert_eq!(low, outer.item())
                            } else {
                                assert_eq!(high, outer.item())
                            }
                        })
                        .is_none());
                }

                #[test]
                fn is_negative() {
                    assert!(negative::<$t>().check(|value| value <= 0 as $t).is_none());
                }

                #[test]
                fn check_finds_maximum() {
                    let fail = (negative::<$t>(), negative::<$t>().keep())
                        .check(|(left, right)| left > right)
                        .unwrap();
                    assert_eq!(fail.item.0, fail.item.1);
                }

                #[test]
                fn check_finds_minimum() {
                    let fail = (positive::<$t>(), positive::<$t>().keep())
                        .check(|(left, right)| left < right)
                        .unwrap();
                    assert_eq!(fail.item.0, fail.item.1);
                }

                #[test]
                fn check_shrinks_irrelevant_items() {
                    let fail = (positive::<$t>(), positive::<$t>().keep(), number::<$t>())
                        .check(|(left, right, _)| left < right)
                        .unwrap();
                    assert_eq!(fail.item.2, 0 as $t);
                }

                #[test]
                fn check_shrink_converges_to_zero() {
                    let mut count = 100usize;
                    let fail = number::<$t>()
                        .check(|_| {
                            count = count.saturating_sub(1);
                            count > 0
                        })
                        .unwrap();
                    assert_eq!(0 as $t, fail.item);
                }
            }
        };
        ($($t:ident),+) => { $(tests!($t);)* };
    }

    tests!(
        i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
    );
}

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check(positive::<u8>(), 0u8)]
    #[check(positive::<u16>(), 0u16)]
    #[check(positive::<u32>(), 0u32)]
    #[check(positive::<u64>(), 0u64)]
    #[check(positive::<u128>(), 0u128)]
    #[check(positive::<usize>(), 0usize)]
    #[check(positive::<i8>(), 0i8)]
    #[check(positive::<i16>(), 0i16)]
    #[check(positive::<i32>(), 0i32)]
    #[check(positive::<i64>(), 0i64)]
    #[check(positive::<i128>(), 0i128)]
    #[check(positive::<isize>(), 0isize)]
    #[check(positive::<f32>(), 0f32)]
    #[check(positive::<f64>(), 0f64)]
    fn is_positive<T: PartialOrd>(value: T, zero: T) {
        assert!(value >= zero);
    }

    #[check(negative::<u8>(), 0u8)]
    #[check(negative::<u16>(), 0u16)]
    #[check(negative::<u32>(), 0u32)]
    #[check(negative::<u64>(), 0u64)]
    #[check(negative::<u128>(), 0u128)]
    #[check(negative::<usize>(), 0usize)]
    #[check(negative::<i8>(), 0i8)]
    #[check(negative::<i16>(), 0i16)]
    #[check(negative::<i32>(), 0i32)]
    #[check(negative::<i64>(), 0i64)]
    #[check(negative::<i128>(), 0i128)]
    #[check(negative::<isize>(), 0isize)]
    #[check(negative::<f32>(), 0f32)]
    #[check(negative::<f64>(), 0f64)]
    fn is_negative<T: PartialOrd>(value: T, zero: T) {
        assert!(value <= zero);
    }
}
