# D7 QA Report

## Package-level checks performed before archive creation

- cumulative D1–D6 specification/structure regression
- D7 JSON metadata validation
- Cargo/TOML and lockfile consistency
- local-only dependency graph
- Rust 1.74 compatibility marker scan
- Rust delimiter and archive-corruption scan
- shell syntax validation
- Python tool compilation
- 5 valid and 10 invalid fixture inventory
- NOM001–NOM010 source/CLI coverage
- 96 Rust test inventory
- record-expression versus `if`-block ambiguity guard
- D6 dependency and reserved-keyword regression guards
- clean ZIP extraction and CRC verification

## Authoritative compile gate

The included GitHub Actions workflow performs the authoritative:

- `cargo metadata --locked`
- `cargo check --workspace --all-targets --locked`
- `cargo test --workspace --all-targets --locked --no-fail-fast`
- cumulative `verify.sh`
- release build
- D7 CLI smoke suite
- artifact/report generation

The delivery is not declared user-passed until that workflow and phone checks are green.

## Compile-risk audit

- method signature-to-body span mapping normalized to the declaration span
- dedicated method return-type regression test added
- record-expression versus `if`-block ambiguity regression retained
