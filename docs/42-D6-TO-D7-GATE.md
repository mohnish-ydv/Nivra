# D6 to D7 Gate

D7 may begin only after:

1. GitHub Actions `Verify D6 Type Checker` is green.
2. `bash scripts/termux-verify.sh` ends with the D6 golden-build marker.
3. all valid D6 fixtures type-check with zero errors.
4. each invalid fixture emits its assigned `TYP` code.
5. `nivra typecheck --functions --types` produces readable reports.
6. JSON type output parses successfully.
7. the user reports `GG D6 Passed`.

D7 should add richer nominal/member typing, generic substitution and constraints,
trait conformance foundations, and stronger flow-sensitive optional handling.
