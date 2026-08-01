# Changelog

## D4 — 2026-08-02

### Added

- `nivra-syntax` lossless CST and typed AST crate
- `nivra-parser` recursive-descent and Pratt parser crate
- 60 syntax node kinds
- lossless source reconstruction
- parser recovery and five `PAR` diagnostics
- `nivra parse` summary, tree, trivia, and JSON modes
- syntax validation in `nivra check`
- five valid and four invalid D4 parser fixtures
- parser/AST unit and CLI integration tests
- D4 GitHub Actions release workflow and Linux artifact
- cumulative D4 report, QA report, and phone verification guide

### Changed

- workspace version from 0.3.0 to 0.4.0
- CLI status from lexer foundation D3 to parser/AST foundation D4
- cumulative verifier now includes D4 structure, parser tests, and smoke tests
- Termux verification destination changed to `~/nivra-d4-verification`

### Preserved

- D1 mission and constitution
- D2 architecture and grammar
- D3 source, diagnostics, and lexer behavior
- D2 fixed temporary-file permission regression

## D3 — 2026-08-02

- Rust compiler workspace, source manager, diagnostics, lexer, and CLI foundation

## D2 — 2026-08-02

- language architecture and Edition 2026 specification draft

## D1 — 2026-08-02

- mission, pain map, constitution, and syntax direction
