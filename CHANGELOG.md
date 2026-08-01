# Changelog

## D6 — 2026-08-02

### Added

- `nivra-types` static type-checker crate
- primitive, nominal, optional, reference, pointer, tuple, and function types
- function signature collection
- local binding inference
- operator, call, condition, array, assignment, and return checking
- `TYP001`–`TYP010` diagnostics
- `nivra typecheck` human/JSON reports
- D6 examples, conformance fixtures, CI, reports, and phone verification

### Changed

- workspace version from 0.5.0 to 0.6.0
- `nivra check` now runs lexical, syntax, semantic, and type phases

## D5 — 2026-08-02

### Added

- `nivra-sema` semantic-analysis crate
- typed semantic symbols, scopes, namespaces, visibility, and resolutions
- module/import/declaration indexing
- lexical name resolution and nearest-name suggestions
- six `SEM` diagnostics
- `nivra resolve` summary, symbol, scope, and JSON views
- semantic phase in `nivra check`
- D5 examples, tests, reports, CI, and Termux checks

### Changed

- workspace version from 0.4.0 to 0.5.0
- typed AST accessor surface expanded
- cumulative workflows now verify D1 through D5

## D4 — 2026-08-02

- lossless CST parser, Pratt expressions, recovery, and AST foundation

## D3 — 2026-08-02

- compiler workspace, source manager, diagnostics, lexer, and CLI

## D2 — 2026-08-02

- architecture and specification draft

## D1 — 2026-08-02

- mission, pain map, constitution, and syntax direction
