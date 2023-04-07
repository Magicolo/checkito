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
                    for i in 1..COUNT {
                        <$t>::generator().samples(i).next().unwrap();
                    }
                }

                #[test]
                fn sample_has_count() {
                    for i in 0..COUNT {
                        assert_eq!(<$t>::generator().samples(i).len(), i);
                    }
                }

                #[test]
                #[should_panic]
                fn empty_range() {
                    <$t>::generator().bind(|value| value..value).check(COUNT, |_| true).unwrap();
                }

                #[test]
                fn is_same() -> Result {
                    number::<$t>().bind(|value| (value, Same(value))).check(COUNT, |&(left, right)| left == right)?;
                    Ok(())
                }

                #[test]
                fn is_in_range() -> Result {
                    (number::<$t>(), number::<$t>())
                        .map(|(low, high)| (low.min($t::MAX - $t::MAX / 100 as $t), high.min($t::MAX - $t::MAX / 100 as $t)))
                        .map(|(low, high)| (low.min(high), low.max(high) + $t::MAX / 100 as $t))
                        .bind(|(low, high)| (low..high, low, high))
                        .check(COUNT, |&(value, low, high)| value >= low && value < high)?;
                    Ok(())
                }

                #[test]
                fn is_in_range_inclusive() -> Result {
                    (number::<$t>(), number::<$t>())
                        .map(|(low, high)| (low.min(high), low.max(high)))
                        .bind(|(low, high)| (low..=high, low, high))
                        .check(COUNT, |&(value, low, high)| value >= low && value <= high)?;
                    Ok(())
                }

                #[test]
                fn is_in_range_from() -> Result {
                    number::<$t>().bind(|low| (low, low..)).check(COUNT, |&(low, high)| low <= high)?;
                    Ok(())
                }

                #[test]
                fn is_in_range_to() -> Result {
                    number::<$t>()
                        .map(|high| high.max($t::MIN + $t::MAX / 100 as $t))
                        .bind(|high| (..high, high))
                        .check(COUNT, |&(low, high)| low < high)?;
                    Ok(())
                }

                #[test]
                fn is_in_range_to_inclusive() -> Result {
                    number::<$t>().bind(|high| (..=high, high)).check(COUNT, |&(low, high)| low <= high)?;
                    Ok(())
                }

                #[test]
                fn is_positive() -> Result {
                    positive::<$t>().check(COUNT, |&value| value >= 0 as $t)?;
                    Ok(())
                }

                #[test]
                fn keeps_value() -> Result {
                    match number::<$t>().keep().check(COUNT, |&value| value < 100 as $t) {
                        Err(error) if error.original() == error.shrunk() => Ok(()),
                        result => result,
                    }?;
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
                    .check(COUNT, |&(left, right)| left < right)
                {
                    Err(error) => {
                        let &(left, right) = error.shrunk();
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
                    .check(COUNT, |&(left, right, _)| left < right)
                {
                    Err(error) if error.shrunk().2 == 0 as $t => Ok(()),
                    result => result,
                }?;
                Ok(())
            }

            #[test]
            fn check_shrink_converges_to_zero() {
                let mut count = COUNT;
                let error = number::<$t>()
                    .check(COUNT, |_| {
                        count = count.saturating_sub(1);
                        count > 0
                    })
                    .unwrap_err();
                assert_eq!(0 as $t, *error.shrunk());
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
                negative::<$t>().check(COUNT, |&value| value <= 0 as $t)?;
                Ok(())
            }

            #[test]
            fn check_finds_maximum() -> Result {
                match (negative::<$t>(), negative::<$t>().keep())
                    .check(COUNT, |&(left, right)| left > right)
                {
                    Err(error) => {
                        let &(left, right) = error.shrunk();
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
                negative::<$t>().check(COUNT, |&value| value <= 0 as $t)?;
                Ok(())
            }
        };
        ($($t:ident),*) => { $(tests!($t, [tests_floating]);)* };
    }

    tests_signed!(i8, i16, i32, i64, i128);
    tests_unsigned!(u8, u16, u32, u64, u128);
    tests_floating!(f32, f64);
}
