# Nivra D9 Implementation Report

## Architecture

D9 adds `nivra-ownership` as a separate local crate after D8 type checking. It consumes the lossless CST and `TypeCheckResult`; it does not rewrite parsing, semantic resolution, nominal typing, generic substitution, or trait checking.

## Implemented model

- `OwnershipClass`: Copy or Move
- `ValueState`: Available, Moved, or MaybeMoved
- place-sensitive whole/field/index move tracking
- shared/mutable loans with overlap checks
- last-use and reference-scope loan expiry
- local borrowed-return origin tracking
- deterministic ownership events and exit actions
- deferred operations before reverse local drops
- conditional drop flags for control-flow/partial moves

Concrete generic arguments are substituted into nominal fields and enum payloads before Copy and drop decisions. Mutable references remain move-only but do not receive drop actions. Borrowed record fields and enum payloads are rejected under the Edition 2026 lifetime-free local-borrow design.

## CLI and diagnostics

`nivra check` runs D9 after a successful type check. `nivra ownership` emits bindings, events, drops, or JSON. The diagnostic inventory is OWN001, OWN002, OWN006, OWN007, and BOR001 through BOR009, with `nivra explain` coverage and one invalid fixture per code.

## Boundary

D9 emits an analysis plan only. HIR/MIR, executable drop glue, code generation, closure capture lowering, interprocedural lifetime summaries, and sendability are not claimed.
