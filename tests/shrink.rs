use checkito::*;

const COUNT: usize = 1024;

#[test]
fn finds_minimum() {
    let result = <(usize, usize)>::generator().check(COUNT, |&(left, right)| left >= right);
    let error = result.err().unwrap();
    assert_eq!(*error.shrunk(), (0, 1));
}
