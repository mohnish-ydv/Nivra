# D8 Delivery Report

## Delivery

- Delivery: D8
- Version: 0.8.0
- Builds on: verified D7
- Scope: generic substitution and non-generic trait constraints
- Workspace: eight local Rust crates
- Third-party Rust dependencies: none
- Rust policy: 1.74

## Implemented outcomes

1. Generic-parameter and generic-argument CST support.
2. Generic functions with explicit and local inference.
3. Generic records, structs, enums, and implementation targets.
4. Recursive substitution through all D8 type forms.
5. Inline trait bounds and `where` clauses.
6. Required and default trait methods.
7. Trait implementation indexing and `Self` replacement.
8. Required-method and signature validation.
9. Exact-pattern coherence and package orphan checking.
10. Deterministic method lookup with ambiguity diagnostics.
11. Twelve new diagnostic codes and dedicated fixtures.
12. Human/JSON reports, CLI smoke tests, and cumulative CI.

## Reliability changes after D7

D8 does not make committed formatting a pre-compilation failure. The runner first
executes `cargo fmt --all`, then compiles every workspace target. Focused parser,
type, and CLI regressions run before the complete `--no-fail-fast` suite. This
prevents a cosmetic formatting mismatch from hiding compiler or behavior failures.

## Deliberate boundaries

- Generic traits and generic trait methods emit GEN006.
- Coherence checks exact canonical target patterns, not specialization overlap.
- Trait-qualified call syntax is not yet available.
- Ownership/move/borrow flow is not part of D8.
- HIR, MIR, C11 generation, LLVM generation, and executable output are future work.

## Verification evidence bundled

The archive includes structural validation for all cumulative deliveries, Cargo
and lockfile graph checks, Rust lexical integrity checks, 135 test declarations,
17 D8 fixtures, workflow contracts, shell/Python validation, and clean archive
verification. GitHub Actions remains the authoritative Rust 1.74 compile/test gate.
