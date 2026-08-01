# D4 Implementation — Parser and AST Foundation

D4 turns the D3 token stream into a lossless concrete syntax tree and introduces
the first typed AST views. The compiler remains a front-end foundation: it does
not resolve names, check types, lower IR, generate C, or execute Nivra programs.

## Added crates

- `nivra-syntax` — immutable CST nodes, retained tokens, tree rendering, exact
  source reconstruction, and typed AST wrappers
- `nivra-parser` — recursive-descent declaration/statement parser, Pratt
  expression parser, diagnostics, and recovery

The cumulative workspace now contains six zero-registry-dependency Rust crates.

## Operational CLI changes

- `nivra check` now performs lexical and syntactic validation.
- `nivra parse FILE` prints a parser summary and lossless round-trip result.
- `nivra parse FILE --tree` prints the CST.
- `nivra parse FILE --tree --trivia` includes comments and whitespace.
- `nivra parse FILE --json` emits a machine-readable nested tree.
- `nivra explain PARxxx` explains parser diagnostics.

## Acceptance boundary

D4 proves that source can be converted into a recoverable, lossless tree. It does
not claim semantic correctness. D5 begins names, scopes, module indexing, and the
semantic AST/HIR boundary.
