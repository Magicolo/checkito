# Clippy Warning: `#[test]` Attribute in README Doctest

## Summary

Running `cargo clippy` on the library produces one warning:

```
warning: unit tests in doctest are not executed
   --> checkito/src/../../README.md:181:1
    |
181 | #[test]
    | ^^^^^^^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#test_attr_in_doctest
    = note: `#[warn(clippy::test_attr_in_doctest)]` on by default
```

The README is included directly as the crate documentation via `#![doc = include_str!("../../README.md")]` in `lib.rs`, and it contains a doctest code block that includes a `#[test]` attribute.  Clippy warns that `#[test]` inside a doctest has no effect (doctests are not run as `#[test]` functions in the conventional sense; they are wrapped by the doctest harness automatically).

## Affected Code

`README.md` at approximately line 181.

The relevant section is the "Cheat Sheet" example in the README, which shows a `#[test]`
function as part of a documentation code example:

```rust
/// The `#[check]` attribute essentially expands to a call to [`Check::check`]
/// with pretty printing. For some more complex scenarios, it may become more
/// convenient to simply call the [`Check::check`] manually.
#[test]
fn has_even_hundred() {
    (0..100, 200..300, 400..500)
        .any()
        .unify::<i32>()
        .check(|value| assert!((value / 100) % 2 == 0));
}
```

The `#[test]` attribute here is being used as an example of how to write a regular test
function (as opposed to the `#[check]` attribute).  However, inside a doctest, `#[test]` is
meaningless — it does not register the function as a test.

## Why This Is a Problem

1. **Build noise:** The clippy warning appears on every `cargo clippy` run, making it harder
   to spot real warnings.
2. **Misleading example:** A user reading the docs might expect the `#[test]` to have the
   same semantics as it does in a real test module.  It does not.
3. **CI hygiene:** Many projects run `cargo clippy -- -D warnings` in CI, which would make
   this a hard error.

## Fix Plan

### Option A – Suppress with `no_run` or `ignore` fence

Mark the code fence in the README so it is not executed as a doctest:

```markdown
```rust,no_run
#[test]
fn has_even_hundred() { … }
```
```

`no_run` means the snippet is compiled but not executed.  This preserves the visual
appearance and still provides compile-time checking without triggering the Clippy lint.

### Option B – Remove the `#[test]` attribute from the code block

Since the `#[test]` is already inside an `fn`, the example is self-contained and the
attribute is not needed for the documentation to make sense:

```rust
fn has_even_hundred() {
    (0..100, 200..300, 400..500)
        .any()
        .unify::<i32>()
        .check(|value| assert!((value / 100) % 2 == 0));
}
```

This removes the misleading implication while keeping the example accurate (the function
would still need `#[test]` to be run by `cargo test` in a real project, but the example is
already showing calling `.check()` directly, not the `#[check]` macro form).

### Option C – Add `#[allow(clippy::test_attr_in_doctest)]` to the crate root

In `lib.rs`:

```rust
#![allow(clippy::test_attr_in_doctest)]
```

This silences the warning globally.  Not recommended because it could mask future real
violations of this lint.

### Recommended

**Option A** or **Option B** is preferred. If the intent is to show that `#[test]` is required
in a normal test module (as opposed to `#[check]`), Option A (`no_run`) keeps the `#[test]`
visible while preventing the lint.  If the example is equally clear without `#[test]`, Option
B is the minimal change.

## Verification

After applying the fix, run:

```bash
cargo clippy 2>&1 | grep -c "test_attr_in_doctest"
# Expected: 0
```
