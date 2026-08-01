# D5 Delivery Report

## Delivery

- Delivery: D5
- Version: 0.5.0
- Builds on: verified D1, D2, D3, and D4
- Status: READY FOR GITHUB/TERMUX VERIFICATION
- Compiler stage: semantic indexing and lexical name resolution
- Type checker included: No
- External Rust dependencies: 0

## Fixed regression

The old fixed `/tmp/nivra-d1-lint.txt` path remains prohibited by cumulative
verification. Termux verification copies the project into app-owned home storage.

## Implemented

1. Added the `nivra-sema` crate.
2. Added stable `SymbolId` and `ScopeId` values.
3. Added separate value and type namespaces.
4. Added module, import, declaration, extern, field, variant, method, parameter,
   local, closure, loop, match-arm, and task-group indexing.
5. Added parent-linked lexical scope trees.
6. Added module-first declaration indexing and declaration-order locals.
7. Added value-name resolution and source-span resolution records.
8. Added nearby-name suggestions for unresolved values.
9. Added six `SEM` diagnostics with primary and related labels.
10. Added `nivra resolve`, symbol reports, scope reports, and JSON output.
11. Upgraded `nivra check` to run the semantic pass after a clean parse.
12. Added semantic regression fixtures and tests.

## Deliberate boundaries

- Member lookup is deferred.
- Full unknown-type diagnostics are deferred.
- Type inference and type compatibility are deferred.
- Cross-file package loading and authoritative privacy checks are deferred.
- Ownership, borrowing, HIR/MIR, native code generation, and execution are later.

## Next delivery

D6 implements the first type checker: primitive/nominal types, function
signatures, local inference, operator and call checking, and type mismatch errors.
