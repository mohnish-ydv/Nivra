# D6 Acceptance Checklist

## Automated

- [x] D1 specification regression retained.
- [x] D2 architecture regression retained.
- [x] D3 compiler-foundation regression retained.
- [x] D4 parser regression retained.
- [x] D5 semantic regression retained.
- [x] Eight local-only Rust crates are declared.
- [x] `nivra-types` implementation and tests are present.
- [x] Ten unique `TYP` diagnostics are implemented and explained.
- [x] Five valid and ten invalid D6 fixtures are present.
- [x] GitHub Actions runs formatting, all tests, debug/release builds, and D6 smoke.
- [x] Fixed `/tmp` output paths remain forbidden.

## Manual

- [ ] GitHub Actions `Verify D6 Type Checker` is green.
- [ ] Termux verifier prints `★★★★★ D6 GOLDEN BUILD`.
- [ ] Valid complete tour reports zero errors.
- [ ] Type/function reports are readable.
- [ ] `TYP001`, `TYP003`, `TYP007`, and `TYP010` manual checks return exit code 1.
- [ ] Typecheck JSON parses through `python3 -m json.tool`.
- [ ] User reports `GG D6 Passed`.
