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
15. Corrected D9 record fixtures that accidentally used `Type(field: value)` instead of the D7 brace syntax `Type { field: value }`.
16. Added a fixture/source regression gate that rejects this syntax drift before ownership tests run.
17. Removed both Rust warnings reported by the supplied CI run and made warnings fatal in every executable verifier.
18. Moved the complete `--no-fail-fast` suite before focused filters so one failure cannot hide the remainder of the suite.
19. Made formatting checks non-mutating; CI can no longer autoformat a checkout and then pass it.
20. Migrated checkout and artifact workflow actions away from the Node-20-deprecated majors reported in the supplied log.
21. Removed the Termux false-golden path that skipped missing `rustfmt` but still printed the final golden marker.
22. Deferred formatting enforcement until after the full/focused tests, so formatting drift cannot conceal later compiler failures while still failing the job.
23. Excluded caches, staging trees, nested ZIPs, bytecode, and temporary smoke files from fresh-extract archives.
24. Enforced D5–D8 as manual-only so D9 is the sole authoritative push/PR gate rather than running redundant cumulative pipelines.
25. Audited every focused workflow test filter against real Rust test functions and lockfile pinning.
26. Closed a GitHub release-packaging leak where `compileall` could place Python bytecode caches in the generated source ZIP; both packaging exclusions and release-tree linting now enforce source-only artifacts.

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

## Supplied CI evidence

The uploaded failing run provides real executable evidence for the pre-fix archive: Rust 1.74 installation, source validation, Cargo metadata, formatting, full-target compilation, D7 regressions, D8 regressions, focused D9 parser/type regressions, and the first two ownership regressions completed before the third ownership test exposed the record-construction fixture bug. The same run reported two Rust warnings and Node-action deprecations; all are addressed in this build-fix release.

## Verification truthfulness

`rustc`, `cargo`, and `rustfmt` are unavailable in the artifact-building sandbox, and network installation is unavailable. Therefore the corrected archive's compilation, full test suite, release build, and executable smoke suite are not claimed as PASS here. The included GitHub Actions and Termux verifier are strengthened to run the authoritative warning-free executable gates and to expose the complete test result before focused filters.
