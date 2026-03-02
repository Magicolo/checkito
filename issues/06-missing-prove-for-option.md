# Missing `Prove` Implementation for `Option<P: Prove>`

## Summary

The `Prove` trait, which determines whether a property test passes or fails, is only
implemented for `()`, `bool`, and `Result<T, E>`.  There is no implementation for
`Option<P>`, even though `Option` is a very common return type in Rust.  Adding a `Prove`
implementation for `Option<P: Prove>` would allow test functions to return `Option` to signal
optional failures, consistent with the rest of the trait design.

## Affected Code

`checkito/src/prove.rs` – the file currently contains three implementations:

```rust
impl Prove for () { … }
impl Prove for bool { … }
impl<T, E> Prove for Result<T, E> { … }
```

## Motivation

### Current Workaround

Without `Prove for Option`, users must manually handle the `Option` return value, for example:

```rust
// With filter: must handle None explicitly
let gen = (0..100u8).filter(|&x| x % 2 == 0);
gen.check(|maybe| {
    let Some(x) = maybe else { return true };  // skip None (filter miss)
    x % 2 == 0
});
```

### Proposed Semantics

```
Some(value) → delegate to value.prove()
None        → Err(()) (failure — the value was not produced)
```

This makes `Option<bool>` mean:
- `Some(true)` → pass
- `Some(false)` → fail
- `None` → fail

And `Option<Result<T, E>>`:
- `Some(Ok(_))` → pass
- `Some(Err(e))` → fail with error `e`
- `None` → fail with unit error

### Practical Use Cases

1. **Filter + prove in one step:**
   ```rust
   (0..100u8)
       .filter_map(|x| (x % 2 == 0).then_some(x))
       .check(|x: Option<u8>| x.map(|v| v < 50));
   ```

2. **Parsing that may fail:**
   ```rust
   digit()
       .collect::<String>()
       .check(|s: String| s.parse::<u64>().ok().map(|n| n > 0));
   ```

3. **More ergonomic filter usage:**
   ```rust
   (0..100u8)
       .filter(|&x| x > 10)
       .check(|x: Option<u8>| x.filter(|&v| v < 50));
   ```

## Proposed Implementation

```rust
impl<P: Prove> Prove for Option<P> {
    type Proof = P::Proof;
    type Error = Option<P::Error>;

    fn prove(self) -> Result<Self::Proof, Self::Error> {
        match self {
            Some(value) => value.prove().map_err(Some),
            None => Err(None),
        }
    }
}
```

The error type `Option<P::Error>` distinguishes:
- `Err(None)` – the `Option` was `None` (value not produced).
- `Err(Some(e))` – the inner `P::Error` was produced.

### Alternative: Always Use `()` as the Error Type

A simpler variant uses `()` as the error type regardless of the inner `Prove`:

```rust
impl<P: Prove> Prove for Option<P> {
    type Proof = P::Proof;
    type Error = ();

    fn prove(self) -> Result<Self::Proof, Self::Error> {
        self.and_then(|value| value.prove().ok()).ok_or(())
    }
}
```

This loses information about the inner error but is simpler to use in `check` output (the
`Fail.message()` method would just print `()`).

### Recommended Approach

Use the first variant with `Option<P::Error>` to preserve the inner error for diagnostic
purposes.

## Impact

- **Severity:** Low – this is a missing convenience feature, not a bug.
- **Benefit:** Allows more idiomatic code using `filter` and `filter_map` together with
  `check`.

## Test Cases to Add

In `checkito/tests/prelude.rs` or a new `tests/prove.rs`:

```rust
#[test]
fn option_some_true_proves() {
    assert!(Some(true).prove().is_ok());
}

#[test]
fn option_some_false_fails() {
    assert!(Some(false).prove().is_err());
}

#[test]
fn option_none_fails() {
    assert!(<Option<bool>>::None.prove().is_err());
    let err = <Option<bool>>::None.prove().unwrap_err();
    assert_eq!(err, None); // Err(None) distinguishes from Err(Some(..))
}

#[test]
fn option_works_with_check() {
    // filter produces Option<u8>; prove should handle None as pass-through skip
    // or fail, depending on semantics chosen.
    let result = (0u8..100)
        .filter(|&x| x % 2 == 0)
        .check(|x: Option<u8>| x.map(|v| v < 50));
    // Depending on semantics: None → fail means this might find a failure at
    // values > 50 OR at filter misses.
    // Adjust assertion based on chosen semantics.
    let _ = result;
}
```
