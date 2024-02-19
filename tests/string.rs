pub mod common;
use common::*;

#[test]
fn filtered_pair_preserves_inequality() -> Result {
    <(String, String)>::generator()
        .filter(|(left, right)| left != right)
        .check(COUNT, |pair| match pair {
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
        .check(COUNT, |array| match array {
            Some([a, b, c]) => a != b && b != c && a != c,
            None => true,
        })?;
    Ok(())
}
