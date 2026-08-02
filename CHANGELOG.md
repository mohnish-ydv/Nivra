# Changelog

## D8 — 2026-08-02

### Added

- generic functions and nominal types
- explicit and locally inferred generic arguments
- nested generic argument CST support
- recursive generic substitution
- inline and where-clause trait constraints
- required and default trait methods
- implementation validation, coherence, and orphan checks
- deterministic method selection
- GEN001–GEN006 and TRT001–TRT006 diagnostics
- D8 reports, fixtures, smoke tests, and GitHub Actions

### Changed

- workspace version to 0.8.0
- typecheck reports now include traits and implementations
- runner normalizes Rust formatting before compile-all-targets
- complete tests use --no-fail-fast

## D7 Formatting and Release Fix — 2026-08-02

### Fixed

- applied every Rust 1.74 formatting hunk reported by the final uploaded Actions log
- repaired repository-wide `cargo fmt --all -- --check` failure across eight Rust files
- removed Termux verifier behavior that silently formatted source before checking it

### Hardened

- committed source is now checked without mutation in both CI and Termux
- added `D7-FORMAT-RELEASE-FIX-REPORT.md` with the exact failure classification
- retained compile-all-targets, four focused regressions, 98 tests, release build, and CLI smoke gates

## D7 Final Build Fix — 2026-08-02

### Fixed

- NOM001 explanation now explicitly describes an unknown `member`
- empty uppercase nominal construction such as `State { }` now parses as a record expression even when leading trivia is preserved
- enum record-construction syntax now reaches the type checker and emits NOM010

### Hardened

- added lossless empty-constructor parser regression
- added direct CLI NOM010 conformance test
- added four focused root-cause tests before the complete workspace suite
- added Rust formatting verification before compilation
- increased cumulative Rust test inventory to 98
- documented the complete two-failure GitHub log analysis in `D7-FINAL-FIX-REPORT.md`

## D7 Build Fix — 2026-08-02

### Fixed

- escaped the embedded `"done"` string in the unknown enum-variant suggestion test
- repaired the Rust parser errors and invalid string-literal suffix reported by CI
- separated the adjacent D6/D7 test attributes for clean source formatting

### Hardened

- detect invalid identifier-like suffixes after Rust string literals
- guard the corrected `State.redy(\"done\")` regression fixture
- run D7 source preflight before Cargo compilation in GitHub Actions
- document the exact root cause in `D7-BUILD-FIX-REPORT.md`

## D7 — 2026-08-02

### Added

- lossless record-construction CST nodes
- record, struct, and enum body indexing
- field/default metadata and named construction
- inherent and trait implementation methods
- `Self` substitution and mutable receiver validation
- enum unit and tuple-payload variant typing
- NOM001–NOM010 diagnostics
- `nivra typecheck --nominals`
- human and JSON nominal reports
- D7 fixtures, tests, smoke suite, reports, and CI

### Changed

- workspace version to 0.7.0
- `nivra check` now includes nominal/member validation
- D6 workflow moved to manual dispatch; D7 is the active push workflow

### Fixed

- protected the D6 parser dev-dependency and reserved-keyword regressions
- added an ambiguity test so `if value {}` is not parsed as record construction
- normalized function/method signature spans so method bodies cannot skip type checking
- report unknown enum variants through NOM001 with nearest-name suggestions

## D6 Final Build Fix — 2026-08-02

### Fixed

- replaced invalid primitive-inference test binding `ok` with `enabled`
- corrected the corresponding Bool type assertion
- preserved the prior `nivra-parser` test dev-dependency and lockfile fix

### Hardened

- compile every workspace target before tests
- run the complete Rust suite with `--no-fail-fast`
- reject the reserved-keyword fixture through the D6 structure validator
- document both GitHub Actions root causes in `D6-FINAL-BUILD-FIX-REPORT.md`

## D6 Build Fix — 2026-08-02

### Fixed

- declared `nivra-parser` as the test-only dependency used by `nivra-types`
- synchronized the corresponding local dependency edge in `Cargo.lock`
- repaired GitHub Actions Rust `E0432` failure in the `nivra-types` test target

### Added

- whole-workspace Cargo manifest/import/lock dependency validator
- early CI `cargo metadata --locked` dependency gate
- permanent regression checks for undeclared local test dependencies
- build-fix report and corrected phone/GitHub commands

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
