# D7 Formatting and Release Fix Report

## Uploaded GitHub Actions failure

The uploaded run reached the pinned Rust 1.74.0 toolchain, dependency validation,
and the D7 source preflight successfully. It stopped at:

```text
cargo fmt --all -- --check
Process completed with exit code 1
```

The failure was repository-wide Rust formatting drift, not a new compiler or
language-semantic failure. Rustfmt reported 159 formatting hunks across eight Rust
source files.

## Repair

- applied the complete Rust 1.74 rustfmt output from the uploaded Actions log
- formatted all eight affected Rust source files
- preserved the D7 parser, NOM001, NOM010, dependency, and reserved-keyword fixes
- changed the Termux verifier so it no longer auto-formats or mutates the source
- changed the cumulative verifier to check committed formatting only
- retained compile-all-targets, focused regressions, full tests, release build,
  and CLI smoke gates in GitHub Actions

## Evidence retained from the immediately preceding run

Before the latest formatting-only failure, GitHub Actions successfully compiled
all eight workspace crates with Rust 1.74.0. That run exposed only the two D7
behavioral regressions subsequently repaired:

1. NOM001 explanation wording
2. empty enum record-construction parsing and NOM010 reporting

The latest uploaded run then passed dependency and D7 structural preflight and
failed exclusively at rustfmt before compilation.

## Local release audit

The repaired archive passed:

- D1 through D7 structural regressions
- Cargo manifest, lockfile, and local dependency-graph validation
- 98-test inventory
- Python compilation
- Bash syntax validation
- JSON and TOML parsing
- source/archive comparison
- ZIP CRC integrity
- fresh-extraction revalidation

The authoritative execution gate remains the included GitHub Actions workflow,
which uses Rust 1.74.0 and runs formatting, compilation, 98 tests, release build,
and D7 CLI smoke checks.
