use checkito::*;

const COUNT: usize = 1024;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn shrink_number() -> Result {
    // TODO: Review `primitive::shrinked`.
    // TODO: Sized ranges should generate 'small' (closer to zero) values first.
    // TODO: Use nudge in float ranges.
    (f64::EPSILON..f64::MAX).check(COUNT, |&value| value > value.nudge(-1.0))?;
    (f64::EPSILON..f64::MAX).check(COUNT, |&value| value < value.nudge(1.0))?;
    (f64::MIN.nudge(-1.0)..=-f64::EPSILON).check(COUNT, |&value| value < value.nudge(-1.0))?;
    (f64::MIN.nudge(-1.0)..=-f64::EPSILON).check(COUNT, |&value| value > value.nudge(1.0))?;
    // (-1e300..0.0).check(1000, |&low| {
    //     let result = (-1e301..=1e301).check(1000, |&value| value > low);
    //     let error = result.unwrap_err();
    //     let shrunk = *error.shrunk();
    //     equals(shrunk, low)
    // })?;

    fn equals(left: f64, right: f64) -> bool {
        if left < right {
            left.nudge(left.signum()) >= right.nudge(-right.signum())
        } else if left > right {
            left.nudge(-left.signum()) <= right.nudge(right.signum())
        } else {
            true
        }
    }

    // assert_eq!(test(f64::MIN, |value| value > LOW), LOW);
    // assert_eq!(test(f64::MAX, |value| value < HIGH), HIGH);
    // assert_eq!(test(f64::MAX, |value| value < LOW), 0.0);
    // assert_eq!(test(f64::MIN, |value| value > HIGH), 0.0);
    Ok(())
}

#[test]
fn finds_minimum() {
    let result = <(usize, usize)>::generator().check(COUNT, |&(left, right)| left >= right);
    let error = result.err().unwrap();
    assert_eq!(*error.shrunk(), (0, 1));
}
