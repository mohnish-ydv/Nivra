# Nivra D9 — Ownership and Borrow Checker Foundation

Nivra is a serious statically typed, compiled general-purpose language. D9 continues the verified D1–D8 compiler without redesigning earlier crates and adds a separate post-type-check ownership-flow pass.

## Current compiler pipeline

```text
UTF-8 source
  → lossless lexer
  → error-recovering CST parser
  → semantic name resolution
  → static and nominal type checking
  → generic substitution and trait-constraint checking
  → ownership, move, borrow, and deterministic drop analysis
```

The reference backend remains C11 + Clang. Executable code generation is intentionally not part of D9.

## D9 commands

```bash
nivra check file.nva
nivra ownership file.nva --bindings --events --drops
nivra ownership file.nva --json
nivra explain OWN001
nivra explain BOR009
nivra doctor
```

## D9 implementation highlights

- structural `Copy`/`Move` classification, including concrete generic substitution
- explicit `move expression` syntax
- whole-value and field-level moves
- use-after-move, move-while-borrowed, partial-move, and maybe-moved diagnostics
- `var` reinitialization after whole or partial moves
- shared and mutable borrow conflict checking
- last-use local borrow regions without user-written lifetime parameters
- borrowed-return origin checks, including local-reference aliases
- borrowed record fields and enum payloads rejected in Edition 2026
- borrows crossing `await` rejected
- deferred borrows retained until scope exit
- deterministic reverse defer and reverse local-drop planning
- drop actions separated from move classification, so references never receive drop glue
- human-readable and JSON ownership reports
- 9 local Rust crates and zero registry dependencies
- pinned Rust 1.74 GitHub Actions and phone-only Termux verification

## Verification

```bash
bash scripts/termux-verify.sh
```

A fully successful Rust run ends with:

```text
★★★★★ D9 GOLDEN BUILD
```

The archive itself has passed structural, manifest, lockfile, JSON, Python, shell, fixture, workflow, and fresh-extraction checks. This artifact-building sandbox did not contain Rust/Cargo, so Rust compilation and test execution are deliberately not claimed here; GitHub Actions or Termux performs the authoritative executable gate.

## D9 boundary

D9 produces compiler-verifiable ownership events and scope-exit plans. HIR/MIR, executable drop glue, closure capture lowering, interprocedural region summaries, sendability, monomorphization, C11 emission, LLVM emission, and runtime execution remain later milestones.
