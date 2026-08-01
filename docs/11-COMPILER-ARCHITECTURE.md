# Compiler Architecture

## Bootstrap implementation

The first compiler is implemented in stable Rust. Rust provides a safe systems
implementation language, strong testing support, and a practical path to a
single native CLI. Self-hosting is postponed until Nivra is stable enough to
compile its own frontend without slowing language progress.

## Frontend strategy

- hand-written Unicode-aware lexer
- hand-written recursive-descent parser
- Pratt parser for operator precedence
- lossless concrete syntax tree for formatting and IDE edits
- error recovery at declarations, statements, delimiters, and list boundaries
- immutable syntax nodes with source spans

A hand-written frontend is selected to maximize diagnostic control and minimize
bootstrap dependencies.

## Compilation stages

1. source manager and file identity
2. Unicode decoding and line map
3. lexer and trivia capture
4. lossless parser / CST
5. AST lowering
6. module graph and name resolution
7. type, trait, and effect checking
8. HIR construction
9. ownership, borrow, move, and sendability analysis
10. MIR lowering with explicit control flow and drops
11. target-independent optimization
12. backend code generation
13. native tool invocation, linking, and artifact packaging

## Intermediate representations

### CST

Preserves comments, whitespace, malformed regions, and exact tokens for formatter
and language-server operations.

### AST

Represents validated syntactic constructs without formatting trivia.

### HIR

Contains resolved names, explicit generic arguments, inferred types, desugared
surface syntax, trait selections, and error-propagation edges.

### MIR

A typed control-flow graph containing explicit moves, borrows, drops, bounds
checks, overflow checks, task operations, and unsafe capabilities. MIR contains
no C-specific semantics.

## Query and caching model

Compiler computations are pure queries keyed by content hashes and target/profile
inputs. D3 begins with deterministic stages; fine-grained incremental caching is
added only after correctness baselines and dependency tracking exist.

## Diagnostics

Diagnostics are structured data with:

- stable diagnostic code
- severity
- primary span
- labeled secondary spans
- concise explanation
- safe machine-applicable edits when available
- related notes
- human terminal rendering
- JSON rendering for editors and CI

The compiler reports the earliest actionable root cause and suppresses cascades
that add no new information.

## Compiler CLI boundary

Users invoke one command: `nivra`. Compiler internals are subcommands and libraries,
not a second required executable. The CLI controls build, run, test, format, lint,
documentation, packages, targets, and diagnostics.

## Testing

Each stage requires:

- unit tests
- golden diagnostic tests
- parser recovery tests
- property tests where suitable
- negative language tests
- conformance examples
- regression tests for every fixed defect

Fuzzing begins when lexer and parser entry points exist.
