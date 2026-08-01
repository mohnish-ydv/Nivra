# D4 QA Report

## Packaged checks

- cumulative D1–D3 regression checks
- D4 repository and machine-readable spec validation
- six-crate workspace validation
- local-only dependency and lockfile validation
- parser/syntax API anchor checks
- 60 syntax-kind coverage check
- five diagnostic-code coverage check
- 33 cumulative Rust test inventory
- fixture inventory and module-header checks
- fixed `/tmp` regression check
- shell syntax and Python compilation checks
- fresh archive extraction and ZIP CRC verification

## Runtime checks delegated to GitHub/Termux

- Rust formatting
- all workspace unit/integration tests
- debug and release builds
- D4 CLI smoke suite
- valid fixture parse checks
- invalid fixture exit-code and diagnostic checks
- JSON parser output decoding
- Linux x86_64 release artifact

## Honest limitation

The packaging container did not contain Rust/Cargo. Rust source was checked by
structural/lexical gates, but the authoritative compile verdict is the included
GitHub Actions workflow and the user's Termux verification.
