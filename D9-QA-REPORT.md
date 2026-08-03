# Nivra D9 QA Report

## Proactive bugs fixed

1. Corrected branch merging so two moving branches produce `Moved`, while one moving branch produces `MaybeMoved`.
2. Expired inner reference loans when the reference scope closes, even when the owner lives outside that scope.
3. Added missing ownership CLI option parsing and JSON serialization.
4. Updated stale D8 CLI identity expectations to 0.9.0/D9.
5. Added explicit `move` end-to-end through parser, type checker, analyzer, examples, and regressions.
6. Kept deferred borrows live until scope exit instead of ending them at the `defer` call site.
7. Substituted concrete generic arguments before structural Copy, drop, and field-place decisions.
8. Separated move-only classification from actual drop necessity, preventing mutable references from receiving drop actions.
9. Rejected borrowed enum payloads as well as borrowed record fields.
10. Tracked returned local borrows through reference aliases and function tail expressions.
11. Made call analysis locate the `ArgumentList` node rather than assuming a child index.
12. Preserved focused D7/D8 build-fix regressions in the D9 workflow.
13. Added fresh-extraction release verification after ZIP creation.
14. Excluded branch-local binding tails from outer branch and match flow joins.

## Static QA executed

- all D1–D9 structure linters
- nine-crate Cargo manifest/lockfile graph validation
- zero-registry-dependency validation
- JSON and TOML parsing
- Python bytecode compilation
- shell syntax validation
- Rust lexical and delimiter preflight
- diagnostic/fixture one-to-one validation
- workflow focused-test-name audit
- release-content and fresh-extraction static checks

These checks passed in the delivery environment.

## Verification truthfulness

`rustc`, `cargo`, and `rustfmt` were unavailable in the artifact-building sandbox. A temporary GitHub CI branch was attempted, but the connected GitHub integration returned HTTP 403 before any repository change occurred. Therefore no Rust compilation, test, formatting, release-build, or executable-smoke result is claimed as PASS here. The included GitHub Actions and Termux verifier perform those authoritative gates.
