# AGENTS.md — Contributor/Agent Guidelines for `checkito`

This file describes high-level expectations for contributors and coding agents working in this repository.

## Project goals

`checkito` is a property-testing library. Changes should prioritize:

- Correctness of generation and shrinking semantics.
- Predictability and reproducibility of behavior.
- Composability of generators and combinators.
- Maintainability and readability.
- Safety-first implementation choices.

When in doubt, prefer preserving behavioral contracts over speculative optimization.

## Engineering standards

- Keep logic explicit in complex paths; favor clear branch structure over clever compactness.
- Prefer robust error handling over panics in core behavior paths.
- Prefer checked operations and conversion APIs (`from`/`try_from`) over unchecked primitive casts.
- When a conservative fallback is required, make it intentional, documented, and test-covered.
- Add comments for *why* a behavior exists, especially in tricky algorithms.

## Testing expectations (TDD-oriented)

Use a TDD-style workflow for behavior changes:

1. Add or update tests to express the intended behavior.
2. Implement the change.
3. Add regression/edge-case coverage.
4. Re-run relevant test suites and confirm they pass.

Testing guidance:

- Prefer precise assertions over weak “non-empty” checks when deterministic behavior allows it.
- Cover edge cases (boundary values, overflow behavior, unknown/optional data paths, composition effects).
- Run tests relevant to touched areas, and ideally run the full affected package test suite before finalizing.

## Documentation expectations

For non-trivial changes:

- Update inline docs/comments near the logic being changed.
- In PR summaries, clearly state:
  - motivation/root cause,
  - behavior change,
  - fallback behavior (if any),
  - tests that validate the change.

If review feedback indicates confusion, improve local comments and add focused tests.

## Practical pre-submit checklist

- [ ] Behavior is correct and deterministic where expected.
- [ ] Error/fallback handling is explicit and non-surprising.
- [ ] Conversions/arithmetic are safe and intentional.
- [ ] Relevant tests were run and pass.
- [ ] Documentation/comments were updated where needed.
