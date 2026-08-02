# D7 Acceptance Checklist

## Automated

- [x] D1–D6 structural regressions remain valid.
- [x] Workspace and lockfile use version 0.7.0.
- [x] No registry dependency is introduced.
- [x] Record construction has dedicated lossless CST nodes.
- [x] Nominal type, field, variant, and method models exist.
- [x] `Self` substitution and mutable receiver checks exist.
- [x] NOM001–NOM010 are implemented and explained.
- [x] Five valid and ten invalid D7 fixtures are included.
- [x] At least 98 cumulative Rust tests are present.
- [x] GitHub Actions compiles all targets before running all tests.
- [x] CLI smoke tests cover valid programs, every nominal diagnostic, reports, and JSON.
- [x] D6 dependency and reserved-keyword regressions are permanently guarded.
- [x] D7 embedded Rust strings reject invalid literal suffixes before Cargo runs.
- [x] The corrected unknown-variant suggestion fixture contains escaped inner quotes.
- [x] Rust 1.74 formatting output from the uploaded Actions log is fully applied.
- [x] Termux verification checks formatting without modifying committed source.

## Manual

- [ ] `Verify D7 Nominal Members` is green.
- [ ] `bash scripts/termux-verify.sh` prints the D7 golden marker.
- [ ] Complete nominal tour returns zero errors.
- [ ] `--nominals` lists fields, variants, and methods.
- [ ] NOM001, NOM003, NOM007, and NOM008 are manually observed.
- [ ] Nominal JSON passes `python3 -m json.tool`.

## Gate

D8 begins only after the user reports:

```text
GG D7 Passed
```
