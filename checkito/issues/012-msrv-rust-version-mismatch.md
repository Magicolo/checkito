# Issue: Declared MSRV (`rust-version = 1.75`) appears incompatible with current source usage

## Summary
`checkito` declares `rust-version = "1.75"`, but static analysis (`cargo clippy`) reports multiple uses of APIs/const contexts stabilized only in Rust 1.83+. This strongly suggests the crate may fail to compile (or at least fail lint policy) on its declared MSRV.

## Why this is an issue
- **Contract break for users**: `rust-version` is a compatibility promise. If users on 1.75 cannot build, dependency resolution and CI can fail unexpectedly.
- **Ecosystem trust**: downstream crates rely on `rust-version` to decide toolchain selection.
- **Maintenance risk**: MSRV regressions can accumulate silently unless explicitly tested in CI.

## Evidence collected
Running:

```bash
cargo clippy -q
```

produced repeated `clippy::incompatible_msrv` warnings indicating language/library features newer than 1.75, including examples in:
- `checkito/src/primitive.rs` (`char::MIN` in const-generic context)
- `checkito/src/state.rs` (const context usage and APIs stabilized later)
- `checkito/src/utility.rs` (`f32::to_bits`, `f64::from_bits`, `is_nan`, etc. in const contexts)

Even though these are lint warnings, they are strong indicators the code path is not aligned with the declared MSRV promise.

## Suspected root cause
The codebase gradually adopted newer const-stable APIs while `Cargo.toml` `rust-version` remained at `1.75`.

## Scope
- `checkito/Cargo.toml` (`rust-version`)
- Const-heavy numeric/state helpers in:
  - `checkito/src/primitive.rs`
  - `checkito/src/state.rs`
  - `checkito/src/utility.rs`
- Potentially any additional const functions/macros using newly stabilized APIs.

## Investigation / fix plan
1. **Decide policy**: either preserve MSRV 1.75 or raise MSRV to the true minimum required version.
2. **Add CI enforcement**:
   - Add an explicit MSRV job (`cargo +<msrv> check/test`), or
   - run `clippy` with `-W clippy::incompatible_msrv` under configured MSRV.
3. **If preserving 1.75**:
   - Refactor const code paths to avoid APIs unavailable in 1.75 const contexts.
   - Gate newer paths behind version cfgs only if truly necessary and maintainable.
4. **If raising MSRV**:
   - Update `rust-version` in crate manifests.
   - Document rationale in changelog/README.
5. **Regression guard**:
   - Add contributor note on avoiding accidental MSRV bumps.

## Acceptance criteria
- Declared `rust-version` matches actual compilable minimum toolchain.
- CI fails when MSRV drifts again.
- Release notes/documentation clearly communicate the supported Rust version.
