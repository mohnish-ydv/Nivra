# D3 to D4 Gate

D4 may begin only when:

1. GitHub Actions is green.
2. `bash verify.sh` prints `★★★★★ D3 GOLDEN BUILD`.
3. all Rust unit and CLI integration tests pass
4. the valid D3 examples return exit code 0
5. each invalid fixture returns exit code 1 with its expected diagnostic
6. `nivra lex` preserves trivia when requested
7. `nivra check --json` produces parseable JSON
8. D1 and D2 specification regression checks pass
9. the user reports `GG D3 Passed`

D4 target: lossless CST parser, Pratt expression parser, parser recovery, AST
lowering foundation, and syntax diagnostics.
