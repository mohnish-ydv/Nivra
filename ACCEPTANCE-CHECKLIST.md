# D5 Acceptance Checklist

## Automated

- [x] D1–D4 structural regressions remain checked.
- [x] Seven local-only Rust crates are present.
- [x] Workspace version and lockfile are 0.5.0.
- [x] No registry dependency exists.
- [x] Semantic scope, symbol, and resolution models exist.
- [x] Six unique `SEM` diagnostics exist and are explained by the CLI.
- [x] `nivra check` invokes semantics only after a clean parse.
- [x] `nivra resolve` supports summary, symbols, scopes, and JSON.
- [x] Five valid and five invalid D5 fixtures exist.
- [x] Cumulative Rust test inventory exceeds 50 tests.
- [x] Fixed `/tmp` paths and unresolved drafting markers are prohibited.
- [x] GitHub Actions builds/tests debug and release binaries.

## Manual

- [ ] GitHub Actions shows a green check for `Verify D5 Semantic Resolution`.
- [ ] `bash scripts/termux-verify.sh` prints `★★★★★ D5 GOLDEN BUILD`.
- [ ] `nivra doctor` reports D5 semantic components as PASS.
- [ ] All valid D5 examples report zero errors.
- [ ] Invalid fixtures emit expected `SEM` codes.
- [ ] Symbol and scope output is readable on a phone.
- [ ] JSON resolution output decodes with Python.

## Gate

Proceed to D6 only after the user reports:

```text
GG D5 Passed
```
