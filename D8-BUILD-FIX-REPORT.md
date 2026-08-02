# D8 Build-Fix Report

## Uploaded GitHub Actions result

The uploaded workflow log reached Rust 1.74 compilation successfully:

- dependency graph validation passed;
- D8 source preflight passed;
- all workspace targets compiled;
- every focused parser regression passed;
- every focused D8 type regression passed;
- every focused D8 CLI regression passed.

The complete `--no-fail-fast` suite then exposed exactly two remaining failures in
`nivra-types`:

1. `rejects_duplicate_generic_parameters` received semantic `SEM005` before the
   D8 type checker could emit the specified `GEN005` diagnostic.
2. `rejects_unknown_enum_variant_with_suggestion` received no `NOM001` because an
   unknown enum payload-variant call was routed through method-call recovery.

No other test in the uploaded run failed.

## Root-cause fixes

### Diagnostic ownership

Semantic indexing now keeps the first duplicate generic symbol available for name
resolution without emitting the older `SEM005`. The type checker sees the complete
syntax list and emits authoritative `GEN005` with both declaration spans.

### Unknown enum variant calls

Enum payload-call checking now handles a missing variant before method dispatch,
emits `NOM001`, computes the nearest declared variant suggestion, and returns a
recovery error type without swallowing the diagnostic.

## Permanent regressions

The final package adds direct tests for:

- semantic deferral of duplicate generic diagnostics;
- type-checker `GEN005` ownership;
- payload-call `NOM001` suggestion behavior;
- CLI `GEN005` output without `SEM005`;
- CLI `NOM001` output with the `ready` suggestion.

All five run as focused GitHub Actions gates before the complete workspace suite.

## Local packaging evidence

The packaging environment does not expose Cargo, so it cannot honestly claim a new
local Rust execution. The following gates were performed on the patched source and
again on the clean archive extraction:

- D1–D8 cumulative structural validation;
- eight-crate Cargo and lockfile dependency validation;
- Rust lexical/string/delimiter preflight;
- 138-test declaration inventory;
- Python, shell, JSON, TOML, and workflow validation;
- source-to-archive byte comparison and ZIP CRC validation.

The next GitHub Actions run remains the authoritative Rust 1.74 execution gate.
