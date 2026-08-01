# D4 to D5 Gate

D5 may begin only when all of these pass:

1. D1, D2, and D3 regression checks remain green.
2. Rust formatting and all workspace tests pass.
3. `nivra check` accepts every valid D4 fixture.
4. Each invalid D4 fixture emits the expected parser code and non-zero exit.
5. `nivra parse` reports `Lossless round trip: PASS`.
6. CST tree output contains declarations, statements, and precedence nodes.
7. JSON parser output can be decoded by Python.
8. GitHub Actions produces the D4 Linux CLI artifact.
9. Phone/Termux manual verification passes.
10. The user reports `GG D4 Passed`.

D5 scope: semantic AST accessors, module indexing, scopes, symbol tables, duplicate
name diagnostics, unresolved-name diagnostics, and the first `nivra check`
semantic pass. Type checking remains a later delivery.
