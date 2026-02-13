# Issue: README doctest block uses `#[test]`, so the example is not executed as a doctest

## Summary
`cargo clippy -q` reports `clippy::test_attr_in_doctest` for a README code block included as crate docs. A code snippet in the documentation uses `#[test]` inside the doctest block, which means the snippet is not executed as a normal rustdoc example in the intended way.

## Why this is an issue
- **Documentation reliability**: examples can silently drift from real API behavior if they are not executed.
- **False confidence**: readers assume shown snippets are validated by doctest.
- **Contributor confusion**: future refactors may break docs without detection.

## Evidence
Running:

```bash
cargo clippy -q
```

emits warning:
- `unit tests in doctest are not executed`
- points into README content included via `#![doc = include_str!("../../README.md")]`.

## Scope
- Root `README.md` (and mirrored `checkito/README.md` / template source, depending on generation flow).
- Crate docs inclusion in `checkito/src/lib.rs`.

## Fix plan
1. Locate the specific README code block currently written with `#[test]`.
2. Convert it into a standard doctest example (plain example code, optional `#` setup lines).
3. If the snippet is intentionally not runnable, mark it explicitly (`ignore`/`no_run`) and explain why.
4. Ensure README/template sync strategy is respected (if `README.tpl` is source-of-truth, edit there and regenerate).
5. Re-run docs-related checks (`cargo test --doc` and/or clippy lint).

## Acceptance criteria
- No `clippy::test_attr_in_doctest` warning remains.
- The corrected example is either executable as doctest or explicitly documented as non-executable.
- README and crate-level docs remain synchronized.
