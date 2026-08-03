# D9 Delivery Report

## Delivery identity

- Delivery: D9
- Version: 0.9.0
- Builds on: verified D8
- New crate: `nivra-ownership`
- Workspace: nine local Rust crates
- Registry dependencies: none
- Rust policy: 1.74
- Edition: 2026
- Reference backend: C11 + Clang, unchanged

## Implemented outcomes

1. Separate ownership analysis after successful type checking.
2. Structural Copy/Move classification for primitives, references, tuples, optionals, generic nominals, records, and enums.
3. Explicit `move` syntax preserved through parsing and static typing.
4. Whole-value, field, and indexed-place move tracking.
5. Available, moved, and maybe-moved control-flow states.
6. Reinitialization of moved mutable values and fields.
7. Shared/mutable loans with overlap and exclusivity checks.
8. Last-use local borrow regions without source-level lifetime parameters.
9. Return-borrow origin restrictions and local-alias escape detection.
10. Borrow-across-`await` rejection.
11. Deterministic defer/drop plans with conditional-drop metadata.
12. Ownership CLI, JSON graph, diagnostic explanations, examples, tests, reports, and cumulative CI.

## Proactive reliability work

- Corrected the all-branches-moved join algorithm.
- Tied loan expiry to the scope containing the reference, not only the owner.
- Kept deferred borrows live until scope exit.
- Separated drop necessity from move classification.
- Applied generic substitutions before Copy/drop/field-place decisions.
- Covered enum payload borrow storage and returned reference aliases.
- Retained every focused D7/D8 root-cause regression in D9 CI.
- Added release ZIP construction followed by fresh extraction, compile, tests, release build, and smoke tests.

## Verification evidence

The delivery environment executed all D1–D9 Python structure linters, Cargo graph static checks, JSON/TOML parsing, Python compilation, shell syntax checks, Rust lexical/delimiter preflight, workflow-test-name checks, release-content inspection, and fresh-extraction static verification. These passed.

The environment did not provide `rustc`, `cargo`, or `rustfmt`. An isolated GitHub verification branch could not be created because the connected integration returned HTTP 403. Therefore this report does **not** claim that Rust compilation or tests executed successfully. Run the included workflow or `bash scripts/termux-verify.sh` for that final gate.
