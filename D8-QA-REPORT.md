# D8 QA Report

## Package-level gates

- cumulative D1–D8 structural checks
- eight-crate Cargo and lockfile graph validation
- no registry dependencies
- Rust lexical delimiter/string-state scanning
- generic/trait source anchor validation
- twelve diagnostic and fixture mappings
- shell syntax and Python bytecode compilation
- GitHub workflow contract validation
- clean ZIP extraction and CRC verification

## Pre-release defect prevention

The final source audit found and corrected three issues before packaging:

- nested explicit type arguments such as `List<Int>` could lose one closing `>`
  when the lexer represented `>>` as a single token;
- the first `GEN005` CLI fixture could stop in semantic duplicate-name checking
  before the D8 type diagnostic was reached;
- default trait methods were available through generic bounds but not yet selected
  on a concrete implemented type.

Dedicated type-checker and CLI regressions now cover all three paths.


## Uploaded Actions failure closure

The uploaded run had exactly two failures after compile-all-targets and every
focused gate passed. The final source closes both root causes and adds semantic,
type-checker, and CLI regressions for diagnostic precedence and enum-call recovery.

## Runtime gates in GitHub Actions and Termux

- pinned Rust 1.74 toolchain
- runner-side Rust formatting normalization
- `cargo check --workspace --all-targets --locked`
- focused parser/type/CLI regressions
- complete 138-test suite with `--no-fail-fast`
- debug and release workspace builds
- D8 valid/invalid CLI smoke suite
- report and JSON validation

## Honest limitation

The packaging container did not expose a Rust compiler and GitHub connector write
permissions were denied, so this report does not claim an unperformed local Rust
build. The included workflow is the authoritative compiler gate and is ordered so
formatting cannot hide compile or test failures.
