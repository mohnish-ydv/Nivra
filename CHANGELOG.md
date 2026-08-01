# Changelog

## D3 — 2026-08-02

### Fixed

- removed the fixed `/tmp/nivra-d1-lint.txt` path that failed on Termux
- made verification compatible with a project-local Termux home copy

### Added

- Rust workspace and Cargo lockfile
- source manager and Unicode-aware line maps
- structured human and JSON diagnostics
- lossless lexer with recovery
- all D2 keywords and core punctuation/operators
- nested comments, numeric bases, strings, characters, and escape validation
- bidi-control and NUL diagnostics
- initial `nivra` CLI
- unit, integration, fixture, smoke, and cumulative regression tests
- D3 GitHub Actions release artifact

## D2 — 2026-08-02

### Added

- Nivra pre-1.0 engineering identity and migration record
- complete type-system architecture
- deterministic ownership-lite memory model
- typed recoverable error model
- structured concurrency model
- Rust bootstrap compiler architecture
- backend-neutral HIR and MIR
- C11 + Clang reference backend strategy
- C ABI and FFI safety rules
- package, workspace, lockfile, and build policy
- Edition 2026 compatibility policy
- Language Specification Draft 0.2
- machine-readable D2 rules and EBNF grammar
- eight D2 design examples
- cumulative D1/D2 verification and GitHub Actions

### Changed

- project identity from provisional Trion to Nivra
- no-value type from the earlier directional `Void` spelling to `Unit`
- unsafe syntax from an unnamed block to named capability blocks
- concurrency examples to explicit task-group handles

## D1 — 2026-08-02

- mission and developer pain map
- language constitution
- syntax direction v0.1
- initial specification verification