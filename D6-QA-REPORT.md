# D6 QA Report

## Static package verification

- cumulative D1–D5 files preserved
- Cargo workspace expanded to eight local crates
- Cargo.lock contains eight 0.6.0 packages and no registry checksums
- Rust 1.74 toolchain contract retained
- unsafe Rust forbidden by workspace lint
- D6 diagnostics inventory is unique and complete
- valid/invalid fixture inventory is complete
- shell scripts parse with `bash -n`
- Python tools compile
- JSON and TOML files parse
- fixed `/tmp` regression absent
- ZIP fresh-extraction checks included

## Authoritative compilation

GitHub Actions and Termux execute actual Rust formatting, compilation, tests,
debug/release builds, CLI smoke tests, and reports. D6 is accepted only after those
checks pass.


## Corrective build audit

- uploaded GitHub Actions log inspected
- failure isolated to undeclared `nivra-parser` test dependency
- `nivra-types/Cargo.toml` and `Cargo.lock` repaired together
- all workspace manifests and local path targets parsed
- all local Rust imports checked against declared dependencies
- all Cargo.lock local edges checked for exact manifest parity
- corrected CI now runs dependency validation before compilation
