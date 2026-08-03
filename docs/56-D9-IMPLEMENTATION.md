# D9 Implementation — Ownership and Borrow Foundation

D9 adds a separate post-type-checking ownership pass without replacing the D1–D8 architecture. The pipeline is now:

`UTF-8 source -> lexer -> CST parser -> semantic resolution -> type checking -> ownership/borrow analysis -> deterministic exit plan`

## New crate

`nivra-ownership` consumes the lossless CST and the successful `TypeCheckResult`. It deliberately does not mutate type checking or hide ownership rules inside the parser. Its output contains binding classifications, source-ordered ownership events, scope-exit actions, and diagnostics.

## Implemented

- structural Copy versus Move classification with concrete generic substitution;
- implicit move at consuming bindings, arguments, returns, tuple/array/record construction;
- explicit `move expression` syntax;
- whole-value and field-level moves;
- use-after-move, move-while-borrowed, partial-move, and maybe-moved diagnostics;
- shared and mutable borrow conflict checks;
- last-use regions for local reference bindings without lifetime syntax;
- conservative joins for if, match, while, and for flow;
- reinitialization of moved mutable places;
- rejection of borrowed record fields/enum payloads, direct or aliased local borrow escape, ambiguous borrowed returns, and borrows across await;
- deferred-borrow retention plus deterministic LIFO defer planning followed by reverse-declaration drops;
- `nivra ownership` and ownership-aware `nivra check`.

## Deliberate D9 boundary

D9 creates a compiler-verifiable ownership plan. HIR/MIR, executable drop glue, closure-capture lowering, interprocedural region summaries, sendability, and code generation remain future milestones. The reference backend remains C11 + Clang.
