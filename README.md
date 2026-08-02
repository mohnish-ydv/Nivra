# Nivra D7 — Nominal Types and Member Checking

> **Final release-fix revision:** includes all earlier D7 compiler/test repairs,
> applies the complete Rust 1.74 rustfmt output reported by GitHub Actions, and
> verifies committed formatting without silently rewriting source files.

Nivra is a statically typed, compiled general-purpose language designed to
deliver native power without recurring developer pain.

D7 turns nominal type names into checked type bodies. The compiler now understands
record and struct fields, record construction, enum variants, inherent methods,
trait implementation methods, `Self`, field mutation, and mutable receivers.

## Current executable pipeline

```text
UTF-8 source
  → lossless lexer
  → error-recovering CST parser
  → semantic name resolution
  → static type checking
  → nominal body and member checking
```

## D7 commands

```bash
nivra check file.nva
nivra typecheck file.nva
nivra typecheck file.nva --functions --types --nominals
nivra typecheck file.nva --json
nivra parse file.nva --tree
nivra explain NOM001
nivra doctor
```

## D7 implementation highlights

- record and struct body indexing
- required and defaulted fields
- named record construction
- field access and field type checking
- mutable field assignment rules
- unit and tuple-payload enum variants
- enum variant arity/type checking
- inherent and trait implementation methods
- `Self` replacement with implementation target
- `&mut Self` receiver enforcement
- nearby-name suggestions for unknown members
- NOM001–NOM010 diagnostics
- human and JSON nominal reports
- 8 zero-third-party-dependency Rust crates
- pinned Rust 1.74 CI
- Android + Termux verification
- 98 cumulative Rust unit/integration tests

## Verify

```bash
bash scripts/termux-verify.sh
```

Expected final marker:

```text
★★★★★ D7 GOLDEN BUILD
```

## Deliberate boundaries

D7 does not yet claim full generic substitution, cross-module privacy,
trait-selection solving, ownership-flow checking, HIR/MIR, code generation, or
execution. Those remain gated future work.
