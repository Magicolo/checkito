# Cardinality Feature Issues

This folder contains detailed documentation of issues found during comprehensive experimentation with the cardinality feature in checkito.

## Quick Navigation

- **[00-SUMMARY.md](00-SUMMARY.md)** - Start here! Comprehensive overview of all experiments and findings
- **[02-filter-cardinality-incorrect.md](02-filter-cardinality-incorrect.md)** - ⚠️ HIGH SEVERITY
- **[01-char-cardinality-includes-surrogates.md](01-char-cardinality-includes-surrogates.md)** - Medium severity
- **[03-lazy-cardinality-incorrect.md](03-lazy-cardinality-incorrect.md)** - Medium severity
- **[04-invalid-range-cardinality.md](04-invalid-range-cardinality.md)** - Medium severity
- **[05-float-cardinality-mismatch.md](05-float-cardinality-mismatch.md)** - Documentation needed

## Issue Priority

### High Priority (Fix Immediately)
1. **Filter Cardinality** (#02) - Breaks correctness guarantees

### Medium Priority (Fix Soon)
2. **Char Cardinality** (#01) - Off by 2,048 values
3. **Lazy Cardinality** (#03) - Returns wrong value
4. **Invalid Range** (#04) - Should return Some(0)

### Low Priority (Document)
5. **Float Cardinality** (#05) - Explain the calculation

## Testing Methodology

All issues were found through systematic experimentation from a user's perspective:
- No access to internals
- Comprehensive edge case testing
- Mathematical boundary testing
- Combinator interaction testing
- Type system limits testing

## Experiments Conducted

The following experiment files were created, run, and then cleaned up (removed):
1. `cardinality_experiments.rs` - Basic primitives and types
2. `char_cardinality_deep_dive.rs` - Char-specific edge cases
3. `collection_cardinality.rs` - Vec, String, nested collections
4. `wrapper_cardinality.rs` - Wrapper type testing
5. `lazy_investigation.rs` - Deep dive into lazy bug
6. `math_edge_cases.rs` - Mathematical overflow and edge cases
7. `float_cardinality_investigation.rs` - Float-specific analysis
8. `any_combinator_tests.rs` - Any combinator edge cases

All experiments ran successfully and produced the documented results.

## Key Findings

### What's Broken
- Filter combinator (high severity)
- Char cardinality calculation
- Lazy wrapper delegation
- Invalid range handling

### What Works Well
- Overflow detection and handling
- Collection cardinality formulas
- Any combinator sum logic
- Most wrapper types
- Composite type products/powers

## For Repository Maintainers

Each issue file includes:
- Type and severity classification
- Detailed description
- Expected vs actual behavior
- Reproduction code
- Impact analysis
- Root cause analysis (where determined)
- Suggested fixes
- Related concerns

## For Users

If you rely on the cardinality feature:
1. **Avoid `filter()`** until fixed - it reports incorrect cardinality
2. **Avoid `lazy()`** if you need accurate cardinality
3. **Be aware** char cardinality is slightly off (by 2,048)
4. **Don't rely** on cardinality to validate ranges (invalid ranges report non-zero)
5. **Float cardinality** is intentional but not documented

## Running Tests

The original experiments have been cleaned up as described. To verify the issues:
1. Review the reproduction code in each issue file
2. The existing cardinality tests in `checkito/tests/cardinality.rs` may not cover these edge cases
3: Consider adding regression tests for each issue once fixed
