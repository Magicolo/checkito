use checkito::*;
#[cfg(test)]
use checkito::state::Weight;

/// The `#[check]` attribute is designed to be as thin as possible and
/// everything that is expressible with it is also ergonomically expressible as
/// _regular_ code (see below). Each `#[check]` attribute expands to a single
/// function call.
///
/// An empty `#[check]` attribute acts just like `#[test]`. It is allowed for
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

/// Regexes can be used and validated either dynamically using the [`regex`]
/// generator or at compile-time with the [`regex!`] macro.
///
/// Usual panicking assertions can be used in the body of the checking function
/// since a panic is considered a failed property.
#[check(regex("{", None).ok(), regex!("[a-zA-Z0-9_]*"))]
fn is_ascii(invalid: Option<String>, valid: String) {
    assert!(invalid.is_none());
    assert!(valid.is_ascii());
}

/// The `_` and `..` operators can be used to infer the [`FullGenerate`]
/// generator implementation for a type. Specifically, the `..` operator works
/// the same way as slice match patterns.
///
/// Since this test will panic, `#[should_panic]` can be used in the usual way.
#[check(..)]
#[check(_, _, _, _)]
#[check(negative::<f64>(), ..)]
#[check(.., negative::<i16>())]
#[check(_, .., _)]
#[check(negative::<f64>(), _, .., _, negative::<i16>())]
#[should_panic]
fn is_negative(first: f64, second: i8, third: isize, fourth: i16) {
    assert!(first < 0.0);
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

/// If a generator has a small domain (i.e. its cardinality is less than or
/// equal to `generate.count`), `#[check]` will automatically switch to
/// exhaustive mode and test every possible value instead of sampling randomly.
///
/// This also means that a test with only the `#[check]` attribute (no specified
/// generator) will run exactly once (since the implicit generator is `()` with
/// a cardinality of 1).
///
/// Here, `bool` has only 2 possible values and `0u8..=9` has 10, so every
/// combination (20 total) is tested exhaustively without any extra
/// configuration.
#[check(_, 0u8..=9)]
fn exhaustive_when_small_domain(sign: bool, digit: u8) {
    // Both generators are fully enumerated; no random sampling needed.
    let signed = if sign { digit as i16 } else { -(digit as i16) };
    assert!((-9..=9).contains(&signed));
}

/// Marking a checking function as `async` will automatically have its test
/// values evaluated concurrently. The concurrency level is determined by the
/// system's available parallelism.
///
/// The function body is an ordinary `async` block, so `.await` can be used
/// freely.
#[check(0u64..1000)]
async fn async_check(value: u64) {
    let doubled = async { value * 2 }.await;
    assert!(doubled < 2000);
}

/// The `#[check]` attribute essentially expands to a call to [`Check::check`]
/// with pretty printing. For some more complex scenarios, it may become more
/// convenient to simply call the [`Check::check`] manually.
///
/// The [`Generate::any`] combinator chooses from its inputs. The produced
/// `Or<..>` preserves the information about the choice but here, it can be
/// simply collapsed using [`Generate::unify<T>`].
#[test]
fn has_even_hundred() {
    (0..100, 200..300, 400..500)
        .any()
        .unify::<i32>()
        .check(|value| assert!((value / 100) % 2 == 0));
}

/// [`Generate::flat_map`] creates a generator that depends on a previously
/// generated value. Here, the generated `length` drives how many items
/// the inner [`Vec`] will contain.
///
/// This is useful when two inputs are logically related: for example a
/// container's size determines its contents, or one field constrains another.
#[check(Generate::flat_map(1usize..20, |length| (0i32..100).collect_with::<_, Vec<_>>(length)))]
fn flat_map_dependent_generation(vector: Vec<i32>) {
    assert!(!vector.is_empty());
    assert!(vector.len() < 20);
    assert!(vector.iter().all(|&x| (0..100).contains(&x)));
}

/// [`Generate::filter`] discards values that don't match a predicate.
///
/// Crucially, the result is [`Option<T>`]: `Some` when the predicate passes,
/// `None` when it doesn't. This design avoids hidden retry loops and makes
/// the partial nature of the filter explicit during both generation and
/// shrinking.
#[check(Generate::filter(0i32..100, |&x| x % 2 == 0))]
fn filter_produces_option(value: Option<i32>) -> bool {
    match value {
        Some(x) => x % 2 == 0 && (0..100).contains(&x),
        None => true, // The filter rejected the candidate; this is expected.
    }
}

/// [`Generate::filter_map`] combines filtering and mapping in one step.
/// Like [`Generate::filter`], it produces [`Option<T>`].
#[check(Generate::filter_map(0u32..100, |x| (x % 3 == 0).then(|| x / 3)))]
fn filter_map_combines_filter_and_transform(value: Option<u32>) -> bool {
    match value {
        Some(third) => third < 34,
        None => true,
    }
}

/// [`Generate::keep`] prevents a value from being shrunk.
///
/// When a property fails, `checkito` shrinks all inputs toward simpler values.
/// Wrapping a generator with `.keep()` pins that input in place, so only the
/// *other* inputs are simplified. This is useful for isolating failures when
/// one parameter should be held constant during shrinking.
#[check(0u64..1_000_000, (0u64..1_000_000).keep())]
#[should_panic]
fn keep_prevents_shrinking(shrinkable: u64, kept: u64) {
    // `shrinkable` will be shrunk to the boundary, while `kept` stays as
    // originally generated.
    assert!(shrinkable + kept < 500_000);
}

/// [`Weight`] lets you bias the [`Generate::any`] combinator.
///
/// In random mode, weights control how often each branch is chosen. In
/// exhaustive mode weights are ignored and all branches are fully enumerated.
///
/// Here, "large" numbers are selected 10× more often than "small" ones.
#[test]
fn weighted_choice() {
    let generator = any((
        Weight::new(1.0, 0i32..10),   // "small" — chosen ~10% of the time
        Weight::new(10.0, 90i32..100), // "large" — chosen ~90% of the time
    ))
        .unify::<i32>();

    // Despite the bias, both branches are still reachable.
    assert!(generator.check(|x| (0..100).contains(&x)).is_none());
}

/// [`same`] creates a constant generator (cardinality 1) that never shrinks.
/// It effectively turns a literal value into a [`Generate`] implementation.
///
/// This can be used together with other generators to fix certain inputs while
/// letting the rest vary freely.
#[check(same(42i32), 0i32..100)]
fn same_as_constant_input(fixed: i32, varied: i32) {
    assert_eq!(fixed, 42);
    assert!((0..100).contains(&varied));
}

/// [`Generate::size`] overrides the internal `size` parameter which ranges
/// from `0.0` to `1.0`. Generators use `size` to decide how "large" or
/// "complex" their output should be (e.g. collections use it for length).
///
/// Setting `size` to `1.0` always produces maximum-complexity values; setting
/// it to `0.0` produces the simplest.
#[test]
fn size_controls_output_complexity() {
    let always_large = Generate::collect::<Vec<_>>(0u8..=255).size(|_| 1.0);
    let always_small = Generate::collect::<Vec<_>>(0u8..=255).size(|_| 0.0);

    // At size 1.0, collections are at their largest; at size 0.0, they are
    // empty.
    let large_sample = always_large.sample(0.5);
    let small_sample = always_small.sample(0.5);
    assert!(large_sample.len() >= small_sample.len());
    assert!(small_sample.is_empty());
}

/// [`Generate::convert`] changes a value's type through a [`From`]
/// implementation. Shrinking is preserved through the conversion.
#[check((0u8..=100).convert::<u32>())]
fn convert_preserves_type_and_shrinking(value: u32) -> bool {
    value <= 100
}

/// [`Generate::array`] generates fixed-size arrays by calling the inner
/// generator `N` times. Each element is independently shrinkable.
#[check((1u8..=50).array::<5>())]
fn array_generates_fixed_size(arr: [u8; 5]) -> bool {
    arr.len() == 5 && arr.iter().all(|&x| (1..=50).contains(&x))
}

/// [`Sample`] provides a way to draw random values from a generator
/// *without* running a property test.
///
/// [`Sample::samples`] produces an iterator of progressively larger values.
/// [`Sample::sample`] produces a single value at a specific `size`.
///
/// Reproducible sequences are available through the [`Sampler`](sample::Sampler)
/// API, which exposes a configurable seed.
#[test]
fn sampling_random_values() {
    // Collect 10 random strings. Sizes increase across the iterator.
    let strings: Vec<String> = letter().collect::<String>().samples(10).collect();
    assert_eq!(strings.len(), 10);
    assert!(strings.iter().all(|s| s.chars().all(|c| c.is_ascii_alphabetic())));

    // Reproducible sampling: same seed → same values.
    let mut sampler = (0u32..1000).sampler();
    sampler.seed = 12345;
    sampler.count = 5;
    let first_run: Vec<u32> = sampler.clone().samples().collect();
    let second_run: Vec<u32> = sampler.samples().collect();
    assert_eq!(first_run, second_run);
}

/// `generate.seed` fixes the random seed, making a `#[check]` run fully
/// reproducible. Useful for debugging a specific failure.
#[check(0u64..1_000_000, generate.seed = 42)]
fn reproducible_with_seed(value: u64) -> bool {
    value < 1_000_000
}

/// `generate.exhaustive = true` forces exhaustive enumeration even when the
/// cardinality exceeds `generate.count`. Combined with a low
/// `generate.count`, you can test just the first few exhaustive values.
///
/// `generate.exhaustive = false` forces random sampling even for small
/// domains that would normally be exhaustive.
#[check(0u8..=5, generate.count = 3, generate.exhaustive = true)]
fn forced_exhaustive(value: u8) -> bool {
    value <= 5
}

/// For recursive data structures, [`lazy`], [`Generate::dampen`], and
/// [`Generate::boxed`] work together to prevent infinite type expansion,
/// control exponential growth, and erase the recursive type.
///
/// See `examples/json.rs` for a full worked example. Here is a minimal sketch:
#[test]
fn recursive_generation_sketch() {
    #[allow(dead_code)]
    #[derive(Clone, Debug)]
    enum Tree {
        Leaf(i32),
        Branch(Vec<Tree>),
    }

    impl Tree {
        fn depth(&self) -> usize {
            match self {
                Tree::Leaf(_) => 1,
                Tree::Branch(children) => {
                    1 + children.iter().map(Tree::depth).max().unwrap_or(0)
                }
            }
        }
    }

    fn tree() -> impl Generate<Item = Tree> {
        (
            Generate::map(0i32..100, Tree::Leaf),
            // `lazy` defers the recursive call to avoid infinite recursion.
            // `collect_with(..4)` limits each branch to 0–3 children.
            // `dampen` reduces `size` as depth increases, encouraging base cases.
            // `boxed` erases the infinite recursive type.
            lazy(tree)
                .collect_with(..4usize)
                .dampen()
                .map(Tree::Branch)
                .boxed(),
        )
            .any()
            .unify()
    }

    // Sample a few trees to verify the generator produces bounded structures.
    for t in tree().samples(10) {
        assert!(t.depth() < 100);
    }
}

fn main() {
    // `checkito` comes with a bunch of builtin generators such as this generic
    // number generator. An array of generators will produce an array of values.
    let generator = &[(); 10].map(|_| number::<f64>());

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
