# Nivra — D2 Architecture & Specification

> **Nivra** is the pre-1.0 engineering identity of the language formerly using
> the provisional D1 name **Trion**.
>
> **Mission:** Power without the pain.

Nivra is a statically typed, compiled, general-purpose programming language and
integrated toolchain designed to remove recurring developer headaches while
preserving native performance and low-level control.

## Delivery status

D1 was independently verified on Android + Termux and GitHub Actions. D2 is a
cumulative design delivery that locks the architecture required before compiler
implementation begins.

- **D1:** mission, developer pain map, constitution, syntax direction
- **D2:** identity, type system, memory model, error model, concurrency,
  compiler architecture, backend, ABI/FFI, package model, compatibility policy,
  grammar, and Language Specification Draft 0.2

D2 intentionally contains no compiler binary. The first implementation delivery
starts only after this specification passes its gate.

## Locked technical identity

- Language: `Nivra`
- CLI: `nivra`
- Source extension: `.nva`
- Manifest: `nivra.toml`
- Lockfile: `nivra.lock`
- First edition: `2026`
- Bootstrap compiler implementation: Rust
- Reference native backend: portable C11 compiled with Clang
- Future optimized backend: LLVM, behind the same backend-neutral MIR
- License: Apache-2.0

The name is locked for pre-1.0 engineering continuity, not represented as legal
trademark clearance. `docs/06-IDENTITY-AND-GOVERNANCE.md` records the review.

## Verify

```bash
bash verify.sh
```

Expected ending:

```text
★★★★★ D2 GOLDEN BUILD
```

## Key documents

- `docs/16-LANGUAGE-SPEC-DRAFT.md` — normative specification index
- `docs/07-TYPE-SYSTEM.md`
- `docs/08-MEMORY-MODEL.md`
- `docs/09-ERROR-MODEL.md`
- `docs/10-CONCURRENCY-MODEL.md`
- `docs/11-COMPILER-ARCHITECTURE.md`
- `docs/12-BACKEND-AND-PORTABILITY.md`
- `docs/13-ABI-AND-FFI.md`
- `docs/14-PACKAGE-AND-BUILD-MODEL.md`
- `spec/d2/grammar.ebnf`
- `examples/d2/08_complete_architecture_tour.nva`
