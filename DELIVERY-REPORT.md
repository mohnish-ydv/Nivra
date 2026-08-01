# D6 Build-Fix Delivery Report

## Delivery

- Delivery: D6 corrective release
- Builds on: user-verified D1–D5 and failed D6 Actions evidence
- Version: 0.6.0
- Edition: 2026
- Status before corrected user verification: BUILD-FIX CANDIDATE
- Workspace crates: 8
- Third-party Rust runtime dependencies: 0

## Failure reproduced from logs

GitHub Actions reached `nivra-types` and failed in its test target with Rust
`E0432`: `use nivra_parser::parse` referenced a crate that was not declared in
`nivra-types` dev-dependencies.

## Corrections

1. added `nivra-parser` as a path-only `dev-dependency` of `nivra-types`
2. synchronized the `nivra-types` dependency set in `Cargo.lock`
3. added a generic eight-crate manifest/import/lock dependency validator
4. made CI run dependency validation and `cargo metadata --locked` before tests
5. added the same dependency validation to the cumulative Termux verifier
6. added permanent regression assertions for the exact missing edge
7. documented the failure, fix, push flow, and manual acceptance procedure

## D6 feature scope retained

The corrected archive retains static type representation, signature collection,
local inference, operator/call/condition/array/assignment/return validation,
`TYP001`–`TYP010`, `nivra typecheck`, five valid fixtures, ten invalid fixtures,
and 75 cumulative Rust tests.

## Verification truthfulness

The packaging environment could not install Rust/Cargo. Static whole-repository
checks, manifest/lock consistency, TOML/JSON/Python/shell validation, fixture/test
inventory, archive integrity, and fresh extraction verification are performed
before release. GitHub Actions and Termux are the authoritative compilation and
runtime-test verdicts. D6 is not accepted until both are green.

## Final follow-up correction

The dependency graph fix compiled successfully. A single unit test then failed because it used reserved keyword `ok` as a variable name. The fixture and assertion now use `enabled`, and CI runs all workspace tests with `--no-fail-fast`. See `D6-FINAL-BUILD-FIX-REPORT.md`.
