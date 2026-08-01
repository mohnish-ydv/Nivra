# D6 Build-Fix Acceptance Checklist

## Corrective automated checks

- [x] Uploaded Actions failure was traced to Rust `E0432` in `nivra-types` tests.
- [x] `nivra-parser` is declared under `nivra-types` dev-dependencies.
- [x] `Cargo.lock` contains the matching `nivra-types -> nivra-parser` edge.
- [x] All eight manifests use only local path dependencies.
- [x] Local Rust imports are checked against manifest dependency declarations.
- [x] Cargo.lock local edges are checked against every manifest.
- [x] CI runs dependency lint and `cargo metadata --locked` before Rust tests.
- [x] D1–D6 structural regressions remain enabled.
- [x] Ten unique `TYP` diagnostics and all D6 fixtures remain present.
- [x] Fixed `/tmp` output paths remain forbidden.

## Required acceptance checks

- [ ] Corrected GitHub Actions `Verify D6 Type Checker` is green.
- [ ] Termux verifier prints `★★★★★ D6 GOLDEN BUILD`.
- [ ] Valid complete tour reports zero errors.
- [ ] Type/function reports are readable.
- [ ] `TYP001`, `TYP003`, `TYP007`, and `TYP010` return exit code 1.
- [ ] Typecheck JSON parses through `python3 -m json.tool`.
- [ ] User reports `GG D6 Passed`.
