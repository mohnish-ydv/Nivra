# Nivra D8 — Generics and Trait Constraints

Nivra is a statically typed, compiled general-purpose language designed to offer
native power without recurring developer pain.

D8 extends the verified D7 pipeline with generic functions and nominal types,
explicit and locally inferred type arguments, non-generic traits, inline and
`where` bounds, implementation validation, default trait methods, an orphan rule,
and deterministic method selection.

## Current executable pipeline

```text
UTF-8 source
  → lossless lexer
  → error-recovering CST parser
  → semantic name resolution
  → static type checking
  → nominal body/member checking
  → generic substitution and trait-constraint checking
```

## D8 commands

```bash
nivra check file.nva
nivra typecheck file.nva
nivra typecheck file.nva --functions --types --nominals --traits
nivra typecheck file.nva --json
nivra parse file.nva --tree
nivra explain GEN004
nivra explain TRT003
nivra doctor
```

## D8 implementation highlights

- generic functions, records, structs, enums, and implementation blocks
- explicit generic arguments such as `identity<Int>(7)`
- local inference such as `identity(7)`
- nested generic type parsing, including `Box<List<Int>>`
- recursive substitution through tuples, optionals, references, and function types
- inline bounds and `where` clauses
- required and default trait methods
- `Self` substitution in trait implementations
- implementation signature and required-method validation
- exact-pattern coherence checks and package orphan rule
- inherent-method priority and ambiguity diagnostics
- `GEN001`–`GEN006` and `TRT001`–`TRT006`
- human and JSON generic/trait reports
- 8 zero-third-party-dependency Rust crates
- pinned Rust 1.74 CI
- Android + Termux verification
- 135 cumulative Rust unit/integration tests

## Verify

```bash
bash scripts/termux-verify.sh
```

Expected final marker:

```text
★★★★★ D8 GOLDEN BUILD
```

## Deliberate boundaries

D8 does not silently pretend to support generic traits or generic trait methods;
they emit `GEN006`. Specialization, full overlap solving, higher-kinded types,
ownership-flow analysis, HIR/MIR, monomorphized code generation, and execution
remain gated future work.
