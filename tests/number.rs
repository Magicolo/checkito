pub mod common;
use checkito::same::Same;
use common::*;

mod range {
    use super::*;

    macro_rules! tests {
        ($t:ident, [$($m:ident),*]) => {
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
                #[should_panic]
                fn empty_range() {
                    <$t>::generator().flat_map(|value| value..value).check(|_| true).unwrap();
                }

                #[test]
                fn is_same() -> Result {
                    number::<$t>().flat_map(|value| (value, Same(value))).check(|(left, right)| left == right)?;
                    Ok(())
                }

                #[test]
                fn is_in_range() -> Result {
                    (number::<$t>(), number::<$t>())
                        .map(|(low, high)| (low.min($t::MAX - $t::MAX / 100 as $t), high.min($t::MAX - $t::MAX / 100 as $t)))
                        .map(|(low, high)| (low.min(high), low.max(high) + $t::MAX / 100 as $t))
                        .flat_map(|(low, high)| (low..high, low, high))
                        .check(|(value, low, high)| value >= low && value < high)?;
                    Ok(())
                }

                #[test]
                fn is_in_range_inclusive() -> Result {
                    (number::<$t>(), number::<$t>())
                        .map(|(low, high)| (low.min(high), low.max(high)))
                        .flat_map(|(low, high)| (low..=high, low, high))
                        .check(|(value, low, high)| value >= low && value <= high)?;
                    Ok(())
                }

                #[test]
                fn is_in_range_from() -> Result {
                    number::<$t>().flat_map(|low| (low, low..)).check(|(low, high)| low <= high)?;
                    Ok(())
                }

                #[test]
                fn is_in_range_to() -> Result {
                    number::<$t>()
                        .map(|high| high.max($t::MIN + $t::MAX / 100 as $t))
                        .flat_map(|high| (..high, high))
                        .check(|(low, high)| low < high)?;
                    Ok(())
                }

                #[test]
                fn is_in_range_to_inclusive() -> Result {
                    number::<$t>().flat_map(|high| (..=high, high)).check(|(low, high)| low <= high)?;
                    Ok(())
                }

                #[test]
                fn is_positive() -> Result {
                    positive::<$t>().check(|value| value >= 0 as $t)?;
                    Ok(())
                }

                #[test]
                fn keeps_value() -> Result {
                    match number::<$t>().keep().check(|value| value < 100 as $t) {
                        Err(Error { shrinks: 0, .. }) => Ok(()),
                        result => result,
                    }?;
                    Ok(())
                }

                #[test]
                fn shrinks_to_zero() -> Result {
                    number::<$t>().check(|value| {
                        let mut outer = $t::shrinker(value).unwrap();
                        while let Some(inner) = outer.shrink() {
                            outer = inner;
                        }
                        assert_eq!(0 as $t, outer.item())
                    })?;
                    Ok(())
                }

                #[test]
                fn shrinks_to_low_or_high() -> Result {
                    number::<$t>()
                        .flat_map(|value| {
                            if value < 0 as $t {
                                (value..=value, value..=0 as $t)
                            } else {
                                (0 as $t..=value, value..=value)
                            }
                        })
                        .flat_map(|(low, high)| (low, high, low..=high))
                        .check(|(low, high, value)| {
                            let mut outer = (low..=high).shrinker(value).unwrap();
                            while let Some(inner) = outer.shrink() {
                                outer = inner;
                            }
                            if low >= 0 as $t {
                                assert_eq!(low, outer.item())
                            } else {
                                assert_eq!(high, outer.item())
                            }
                        })?;
                    Ok(())
                }

                $($m!(INNER $t);)*
            }
        };
    }

    macro_rules! tests_integer {
        (INNER $t:ident) => {
            #[test]
            fn check_finds_minimum() -> Result {
                match (positive::<$t>(), positive::<$t>().keep())
                    .check(|(left, right)| left < right)
                {
                    Err(error) => {
                        let (left, right) = error.item;
                        if right - left <= right / 100 as $t {
                            Ok(())
                        } else {
                            Err(error)
                        }
                    }
                    result => result,
                }?;
                Ok(())
            }

            #[test]
            fn check_shrinks_irrelevant_items() -> Result {
                match (positive::<$t>(), positive::<$t>().keep(), positive::<$t>())
                    .check(|(left, right, _)| left < right)
                {
                    Err(error) if error.item.2 == 0 as $t => Ok(()),
                    result => result,
                }?;
                Ok(())
            }

            #[test]
            fn check_shrink_converges_to_zero() {
                let mut count = 100usize;
                let error = number::<$t>()
                    .check(|_| {
                        count = count.saturating_sub(1);
                        count > 0
                    })
                    .unwrap_err();
                assert_eq!(0 as $t, error.item);
            }
        };
        ($t:ident, $m:ident) => {
            tests!($t, [$m, tests_integer]);
        };
    }

    macro_rules! tests_signed {
        (INNER $t:ident) => {
            #[test]
            fn is_negative() -> Result {
                negative::<$t>().check(|value| value <= 0 as $t)?;
                Ok(())
            }

            #[test]
            fn check_finds_maximum() -> Result {
                match (negative::<$t>(), negative::<$t>().keep())
                    .check(|(left, right)| left > right)
                {
                    Err(error) => {
                        let (left, right) = error.item;
                        if left - right <= right.abs() / 100 as $t {
                            Ok(())
                        } else {
                            Err(error)
                        }
                    }
                    result => result,
                }?;
                Ok(())
            }
        };
        ($($t:ident),*) => { $(tests_integer!($t, tests_signed);)* };
    }

    macro_rules! tests_unsigned {
        (INNER $t:ident) => {};
        ($($t:ident),*) => { $(tests_integer!($t, tests_unsigned);)* };
    }

    macro_rules! tests_floating {
        (INNER $t:ident) => {
            #[test]
            fn is_negative() -> Result {
                negative::<$t>().check(|value| value <= 0 as $t)?;
                Ok(())
            }
        };
        ($($t:ident),*) => { $(tests!($t, [tests_floating]);)* };
    }

    tests_signed!(i8, i16, i32, i64, i128);
    tests_unsigned!(u8, u16, u32, u64, u128);
    tests_floating!(f32, f64);
}
