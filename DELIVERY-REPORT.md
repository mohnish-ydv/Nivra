# D4 Delivery Report

## Delivery identity

- Delivery: D4
- Version: 0.4.0
- Builds on: verified D1, D2, and D3
- Status before user verification: RELEASE CANDIDATE
- Primary target: Android + Termux and GitHub Actions
- External Rust registry dependencies: 0

## Fixed regression

D2 used a fixed `/tmp/nivra-d1-lint.txt` path that failed on the user's Termux
environment. D3 removed that path; D4 keeps the regression check and performs all
phone builds from `$HOME/nivra-d4-verification`.

## Implemented

1. Added `nivra-syntax` and `nivra-parser` crates.
2. Added 60 stable CST node kinds.
3. Preserved every lexer token, including whitespace and comments.
4. Added exact lossless source reconstruction.
5. Added recursive-descent declaration and statement parsing.
6. Added Pratt expression parsing with 12 precedence levels.
7. Added calls, members, indexes, closures, async, task-group, unsafe, try,
   await, spawn, if, match, loop statements, and common expressions.
8. Added delimiter and synchronization-based recovery.
9. Added five parser diagnostic codes.
10. Added typed AST views over CST nodes.
11. Upgraded `nivra check` from lexical to lexical + syntax checking.
12. Added `nivra parse` summary, tree, trivia, and JSON modes.
13. Added five valid and four invalid D4 fixtures.
14. Added cumulative Rust tests and CLI smoke tests.
15. Added D4 CI release artifact and parser reports.

## Verification evidence available before GitHub

The release ZIP has passed:

- D1 specification regression
- D2 architecture regression
- D3 structure regression
- D4 machine-readable structure validation
- Cargo manifest parsing
- local-only dependency validation
- Cargo lock validation
- Rust lexical delimiter preflight
- keyword parity and diagnostic coverage checks
- shell syntax checks
- Python tool compilation
- fixture and test inventory checks
- ZIP extraction and integrity checks

## Required external compile verdict

GitHub Actions must still compile and test the Rust workspace. The final delivery
is accepted only after the workflow and manual Termux checks pass. No local Rust
compiler was available in the packaging environment, so this report does not make
a false compile-success claim.

## Next delivery

D5: semantic AST accessors, module indexing, lexical scopes, symbol tables,
duplicate declaration diagnostics, unresolved-name diagnostics, and the first
semantic `nivra check` pass. Type checking remains later.
