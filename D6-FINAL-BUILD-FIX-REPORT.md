# D6 Final Build Fix Report

## User-reported failure

The corrected dependency graph compiled successfully on GitHub Actions. The focused
`nivra-types` test binary then ran all 12 tests:

- 11 passed
- 1 failed: `tests::infers_primitive_bindings`

The failing fixture declared:

```nva
let ok = true
```

`ok` is an Edition 2026 reserved keyword used by the typed-result constructor, so
it is not a valid ordinary binding identifier. The test incorrectly expected the
compiler to create a Bool binding named `ok`.

## Fix

The fixture now uses the valid identifier `enabled`:

```nva
let enabled = true
```

The corresponding assertion now checks `enabled: Bool`.

## Prevention

- `tools/d6_structure_lint.py` rejects the old reserved-keyword fixture.
- CI first compiles every workspace target with `cargo check --all-targets`.
- CI runs the complete workspace suite with `--no-fail-fast`, avoiding a focused
  test gate that can hide failures in later crates.
- The local cumulative verifier uses the same no-fail-fast test mode.

## Previous build fix retained

The earlier missing `nivra-parser` dev-dependency and its `Cargo.lock` edge remain
corrected and are checked by `tools/d6_dependency_lint.py`.

## Validation boundary

The supplied GitHub log proves that the dependency fix compiled `nivra-source`,
`nivra-diagnostics`, `nivra-lexer`, `nivra-syntax`, `nivra-parser`, `nivra-sema`,
and `nivra-types` successfully under Rust 1.74.0 before the single assertion
failure. This archive fixes that exact assertion defect and strengthens the CI
sequence. The next GitHub Actions run remains the authoritative full compile,
workspace-test, release-build, and CLI-smoke verdict.
