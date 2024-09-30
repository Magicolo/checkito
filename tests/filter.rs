pub mod common;
use common::*;

#[test]
fn filtered_pair_preserves_inequality() -> Result {
    <(String, String)>::generator()
        .filter(|(left, right)| left != right)
        .check(|pair| match pair {
            Some((left, right)) => left != right,
            None => true,
        })?;
    Ok(())
}

#[test]
fn filtered_array_preserves_inequality() -> Result {
    regex!("[a-z]+")
        .array::<3>()
        .filter(|[a, b, c]| a != b && b != c && a != c)
        .check(|array| match array {
            Some([a, b, c]) => a != b && b != c && a != c,
            None => true,
        })?;
    Ok(())
}

#[test]
fn shrinked_filter_preserves_inequality() -> Result {
    let error = (
        <(String, String)>::generator().filter(|(left, right)| left != right),
        usize::generator(),
    )
        .check(|(pair, value)| {
            let Some((left, right)) = pair else {
                return true;
            };
            assert_ne!(left, right);
            value < 1000 // Force the check to fail at some point.
        })
        .unwrap_err();
    assert_eq!(error.cause, Cause::Disprove(()));
    let (left, right) = error.item.0.clone().unwrap();
    assert_ne!(left, right);
    Ok(())
}
