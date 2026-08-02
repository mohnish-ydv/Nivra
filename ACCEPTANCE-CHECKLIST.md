# D8 Acceptance Checklist

## Automated

- [x] D1–D7 structural regressions remain valid.
- [x] Workspace and lockfile use version 0.8.0.
- [x] No registry dependency is introduced.
- [x] Generic arguments have a dedicated lossless CST node.
- [x] `<` comparisons are not misclassified as generic arguments.
- [x] Nested closing `>>` is handled in generic lists.
- [x] Generic functions and nominal types support explicit and inferred arguments.
- [x] Inline bounds and `where` constraints are normalized.
- [x] Required/default trait methods and `Self` substitution are modeled.
- [x] Implementation signatures, required methods, coherence, and orphan rules are checked.
- [x] Ambiguous method selection never depends on declaration order.
- [x] Unsupported generic traits/methods fail explicitly with GEN006.
- [x] GEN001–GEN006 and TRT001–TRT006 are implemented and explained.
- [x] Five valid and twelve invalid D8 fixtures are included.
- [x] At least 135 cumulative Rust tests are present.
- [x] GitHub Actions normalizes formatting before compiling every target.
- [x] Focused parser, type-checker, and CLI regressions run before the full suite.
- [x] The complete suite uses `--no-fail-fast`.
- [x] Release build, CLI smoke suite, reports, and JSON validation are gated.

## Manual

- [ ] `Verify D8 Generics and Traits` is green.
- [ ] `bash scripts/termux-verify.sh` prints the D8 golden marker.
- [ ] Complete D8 tour returns zero errors.
- [ ] `--functions --types --nominals --traits` shows substitutions and implementations.
- [ ] GEN001, GEN004, GEN006, TRT003, TRT005, and TRT006 are manually observed.
- [ ] D8 JSON passes `python3 -m json.tool`.

## Gate

D9 begins only after the user reports:

```text
GG D8 Passed
```
