use checkito::*;

/// __HOLD ON!__ Yes, there is an attribute which might repulse some of you by
/// fear of too much macro-magic. The attribute is designed to be as thin as
/// possible and everything that is expressible with the attribute is also
/// ergonomically expressible as _regular_ code (see below). To convince
/// yourself, try running `cargo expand` and you'll see that each `#[check]`
/// attribute expands to a single function call.
///
/// An empty `#[check]` attribute acts just like `#[test]`. It exists for
/// consistency between tests.
#[check]
fn empty() {}

/// The builtin `letter()` generator will yield ascii letters.
///
/// This test will be run many times with different generated values to find a
/// failing input.
#[check(letter())]
fn is_letter(value: char) {
    assert!(value.is_ascii_alphabetic());
}

/// Ranges can be used as generators and will yield values within its bounds.
///
/// A [`bool`] can be returned and if `true`, it will be considered as evidence
/// that the property under test holds.
#[check(0usize..=100)]
fn is_in_range(value: usize) -> bool {
    value <= 100
}

/// Regexes can be used and validated at compile-time with the [`regex!`] macro.
///
/// Usual panicking assertions can be used in the body of the checking function
/// since a panic is considered a failed property.
#[check(regex!("[a-zA-Z0-9_]*"))]
fn is_ascii(value: String) {
    assert!(value.is_ascii());
}

/// The `_` and `..` operators can be used to infer the [`FullGenerate`]
/// generator implementation for a type.
///
/// Since this test will panic, `#[should_panic]` can be used in the usual way.
#[check(_, _, _, _)]
#[check(_, _, ..)]
#[check(..)]
#[should_panic]
fn is_negative(first: f64, second: i8, third: isize, fourth: i16) {
    assert!(first < 0);
    assert!(second < 0);
    assert!(third < 0);
    assert!(fourth < 0);
}

/// `color = false` disables coloring of the output.
/// `verbose = true` will display all the steps taken by the [`check::Checker`]
/// while generating and shrinking values.
///
/// The shrinking process is pretty good at finding minimal inputs to reproduce
/// a failing property and in this case, it will always shrink values over
/// `1000` to exactly `1000`.
#[check(0u64..1_000_000, color = false, verbose = true)]
#[should_panic]
fn is_small(value: u64) {
    assert!(value < 1000);
}

/// Multiple checks can be performed.
///
/// If all generators always yield the same value, the check becomes a
/// parameterized unit test and will run only once.
#[check(3001, 6000)]
#[check(4500, 4501)]
#[check(9000, 1)]
fn sums_to_9001(left: i32, right: i32) {
    assert_eq!(left + right, 9001);
}

/// Generics can be used as inputs to the checking function.
///
/// [`Generate::map`] can be used to map a value to another.
#[check(111119)]
#[check(Generate::map(10..1000, |value| value * 10 - 1))]
#[check("a string that ends with 9")]
#[check(regex!("[a-z]*9"))]
fn ends_with_9(value: impl std::fmt::Display) -> bool {
    format!("{value}").ends_with('9')
}

pub struct Person {
    pub name: String,
    pub age: usize,
}
/// Use tuples to combine generators and build more complex structured types.
/// Alternatively implement the [`FullGenerate`] trait for the [`Person`]
/// struct.
///
/// Any generator combinator can be used here; see the other examples in the
/// _examples_ folder for more details.
///
/// Disable `debug` if a generated type does not implement [`Debug`] which
/// removes the only requirement that `#[check]` requires from input types.
#[check((letter().collect(), 18usize..=100).map(|(name, age)| Person { name, age }), debug = false)]
fn person_has_valid_name_and_is_major(person: Person) {
    assert!(person.name.is_ascii());
    assert!(person.age >= 18);
}

/// The `#[check]` attribute essentially expands to a check to [`Check::check`]
/// with pretty printing. For some more complex scenarios, it becomes more
/// convenient to simply call the [`ChecK::check`] manually.
///
/// The [`Generate::any`] combinator chooses from its inputs. The produced
/// `Or<..>` preserves the information about the choice but here, it can be
/// simply collapsed using `Or<..>::into::<T>()`.
#[test]
fn has_even_hundred() {
    (0..100, 200..300, 400..500)
        .any()
        .map(|or| or.into::<i32>())
        .check(|value| assert!((value / 100) % 2 == 0));
}

fn main() {
    // `checkito` comes with a bunch of builtin generators such as this generic
    // number generator. An array of generators will produce an array of values.
    let generator = [(); 10].map(|_| number::<f64>());

    // For more configuration and control over the generation and shrinking
    // processes, retrieve a [`check::Checker`] from any generator.
    let mut checker = generator.checker();
    checker.generate.count = 1_000_000;
    checker.shrink.items = false;

    // [`check::Checker::checks`] produces an iterator of [`check::Result`] which
    // hold rich information about what happened during each check.
    for result in checker.checks(|values| values.iter().sum::<f64>() < 1000.0) {
        match result {
            check::Result::Pass(_pass) => {}
            check::Result::Shrink(_pass) => {}
            check::Result::Shrunk(_fail) => {}
            check::Result::Fail(_fail) => {}
        }
    }

    // For simply sampling random values from a generator, use [`Sample::samples`].
    // Just like in the checking process, samples will get increasingly larger.
    for _sample in generator.samples(1000) {}
}
