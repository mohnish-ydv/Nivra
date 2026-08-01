# D5 QA Report

## Static package QA

- cumulative specification files retained
- workspace manifests parse as TOML
- dependency graph remains local-only
- Cargo lock contains seven Nivra packages
- semantic implementation anchors present
- semantic diagnostic inventory complete
- valid/invalid fixtures complete
- shell and Python preflight included
- fixed `/tmp` regression prohibited
- clean-extraction verification required before release

## Authoritative compilation QA

GitHub Actions and the user's Termux verifier perform actual Rust formatting,
workspace compilation, all tests, debug/release builds, and CLI smoke tests.
