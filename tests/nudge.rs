use checkito::*;

const COUNT: usize = 1024;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn positive_towards_zero() -> Result {
    (f64::EPSILON..f64::MAX).check(COUNT, |&value| value > value.nudge(-1.0))?;
    Ok(())
}

#[test]
fn positive_towards_maximum() -> Result {
    (f64::EPSILON..f64::MAX).check(COUNT, |&value| value < value.nudge(1.0))?;
    Ok(())
}

#[test]
fn negative_towards_zero() -> Result {
    (f64::MIN.nudge(-1.0)..=-f64::EPSILON).check(COUNT, |&value| value < value.nudge(-1.0))?;
    Ok(())
}

#[test]
fn negative_towards_minimum() -> Result {
    (f64::MIN.nudge(-1.0)..=-f64::EPSILON).check(COUNT, |&value| value > value.nudge(1.0))?;
    Ok(())
}
