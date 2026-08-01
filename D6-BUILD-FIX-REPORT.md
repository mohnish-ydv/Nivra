# D6 GitHub Actions Build-Fix Report

## Reported failure

The uploaded GitHub Actions log failed during:

```text
cargo test --workspace --all-targets --locked
```

Rust compiled the D6 dependency chain through `nivra-types`, then failed while
compiling the `nivra-types` test target:

```text
error[E0432]: unresolved import `nivra_parser`
 --> crates/nivra-types/src/lib.rs:1657:9
  |
  | use nivra_parser::parse;
  |     ^^^^^^^^^^^^ use of undeclared crate or module `nivra_parser`
```

## Root cause

`nivra-types` unit tests call the parser to construct syntax trees, but
`crates/nivra-types/Cargo.toml` did not declare `nivra-parser` as a test-only
dependency. Because CI uses `--locked`, the corresponding edge also had to exist
in `Cargo.lock`.

## Applied correction

```toml
[dev-dependencies]
nivra-parser = { path = "../nivra-parser" }
```

The `nivra-types` package entry in `Cargo.lock` now contains `nivra-parser`.

## Preventive controls added

1. `tools/d6_dependency_lint.py` parses all eight Cargo manifests.
2. Every local dependency must be an isolated path dependency.
3. Every `use nivra_*` import in `src/` and `tests/` must have a manifest edge.
4. Every manifest edge must point to the package named by its dependency key.
5. `Cargo.lock` local dependency sets must exactly match the manifests.
6. GitHub Actions runs this check and `cargo metadata --locked` before tests.
7. The cumulative verifier runs the same dependency check in Termux.

## Audit boundary

The packaging sandbox could not install a Rust toolchain, so it does not claim an
unobserved local Cargo success. The uploaded Actions log, complete source archive,
Cargo manifests, lock graph, Python/TOML/JSON checks, shell syntax, Rust source
preflight, fixture inventory, and clean ZIP extraction were audited. The corrected
GitHub Actions run remains the authoritative Rust compilation and test verdict.
