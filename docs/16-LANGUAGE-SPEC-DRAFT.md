# Nivra Language Specification Draft 0.2

## Status

This is the first architecture-complete draft for Edition 2026. It is normative
for implementation planning but remains pre-release: conformance details may be
refined when the compiler reveals ambiguity. Constitutional rules and D2 locked
architecture require an RFC to change.

## Normative document map

1. `00-MISSION.md` — purpose and success criteria
2. `01-DEVELOPER-PAIN-MAP.md` — product obligations
3. `02-LANGUAGE-CONSTITUTION.md` — governing principles
4. `03-SYNTAX-DIRECTION.md` — D1 surface direction
5. `06-IDENTITY-AND-GOVERNANCE.md` — identity and RFC process
6. `07-TYPE-SYSTEM.md` — types, inference, conversions, traits
7. `08-MEMORY-MODEL.md` — ownership, borrowing, destruction, unsafe code
8. `09-ERROR-MODEL.md` — results, propagation, panic
9. `10-CONCURRENCY-MODEL.md` — tasks, cancellation, race safety
10. `11-COMPILER-ARCHITECTURE.md` — compilation stages and diagnostics
11. `12-BACKEND-AND-PORTABILITY.md` — code generation and targets
12. `13-ABI-AND-FFI.md` — foreign boundaries
13. `14-PACKAGE-AND-BUILD-MODEL.md` — project and dependency semantics
14. `15-COMPATIBILITY-AND-EDITIONS.md` — evolution policy
15. `spec/d2/grammar.ebnf` — surface grammar draft

## Lexical summary

- UTF-8 source
- Unicode identifiers with normalized comparison policy finalized by the compiler
  implementation before public source stability
- braces delimit blocks
- semicolons optional at line endings
- line and nested block comments
- documentation comments beginning with `///`
- string interpolation through `${expression}`
- no preprocessor

## Declaration summary

Nivra supports modules, imports, constants, functions, records, structs, enums,
newtypes, aliases, traits, implementations, and C foreign blocks. Declarations are
private unless marked `pub`.

## Expression summary

Expressions include literals, names, tuples, arrays, records, calls, member access,
indexing, unary/binary operations, blocks, `if`, `match`, loops, closures, `try`,
`await`, task groups, and named unsafe blocks.

## Semantic invariants

- no ambient null
- no general truthiness
- immutable bindings by default
- checked integer arithmetic in all standard profiles
- explicit numeric conversion
- move-by-default for non-copy values
- deterministic destruction on normal exits
- no borrow crossing `await`
- no data race in safe code
- recoverable failure through typed results
- panic aborts and is not catchable
- unsafe behavior is capability-labeled
- class inheritance is absent
- common workflow uses one official CLI
- application dependencies are reproducibly locked

## Standard project contract

A conforming application uses `nivra.toml`, commits `nivra.lock`, declares edition
2026, and can be checked through `nivra check`. Generated artifacts are target and
profile specific; source semantics remain target-independent except behind explicit
platform capability checks.

## Implementation-resolution rule

When this draft leaves a detail unspecified, the compiler must not silently create
a permanent rule. The ambiguity must be recorded as a specification issue, resolved
through a decision record, and covered by a conformance test before stable release.

## Edition 2026 future exclusions

The first edition excludes class inheritance, explicit lifetime parameters,
borrowed fields, catchable exceptions, catchable panic, arbitrary package install
scripts, C++ ABI promises, generic specialization, higher-kinded types, and a
mandatory tracing garbage collector.

## D4 implementation status

The Edition 2026 draft now has an executable lossless parser foundation. D4
implements declaration, statement, type/pattern shell, and Pratt expression CST
construction with recovery. Parser acceptance is not yet language conformance:
semantic validity begins with D5 name and scope analysis.


## D5 implementation status

D5 adds the first semantic pass above the lossless parser. It creates module,
lexical-scope, symbol, namespace, visibility, and source-name resolution data.
Value names are resolved in declaration order inside nested scopes; module
declarations are indexed before function bodies. Type-name completeness, member
lookup, overload selection, and type compatibility remain D6 responsibilities.
