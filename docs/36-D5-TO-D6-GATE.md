# D5 to D6 Gate

D6 may begin only when:

1. D1–D4 regression checks pass.
2. All workspace tests pass with the locked dependency graph.
3. Every valid D5 fixture reports zero semantic errors.
4. Invalid fixtures emit their expected `SEM` codes.
5. `nivra resolve --symbols --scopes` shows a stable module graph.
6. `nivra resolve --json` is machine-decodable.
7. Existing D2–D4 valid examples remain accepted by `nivra check`.
8. GitHub Actions produces the D5 Linux CLI artifact.
9. Phone/Termux manual verification passes.
10. The user reports `GG D5 Passed`.

D6 scope: primitive and nominal type representation, function signatures, local
type inference, operator checking, call checking, type mismatch diagnostics, and
the first typed semantic model.
