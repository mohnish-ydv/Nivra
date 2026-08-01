# D4 Acceptance Checklist

## Automated release gates

- [x] D1 specification regression passes.
- [x] D2 architecture regression passes.
- [x] D3 compiler-foundation structure regression passes.
- [x] D4 specification and repository structure pass.
- [x] Workspace contains six local-only Rust crates.
- [x] Cargo manifests and lockfile are structurally valid.
- [x] Lossless CST and typed AST implementation anchors exist.
- [x] All 60 required syntax node kinds are represented.
- [x] Five parser diagnostic codes are implemented/explained.
- [x] Five valid and four invalid parser fixtures exist.
- [x] At least 30 cumulative Rust tests exist.
- [x] Fixed D2 temporary path does not return.
- [x] GitHub Actions invokes the cumulative verifier and release build.

## User compile and runtime gates

- [ ] GitHub Actions `Verify D4 Parser and AST` is green.
- [ ] `bash scripts/termux-verify.sh` prints the D4 golden-build marker.
- [ ] `nivra doctor` reports every D4 subsystem as PASS.
- [ ] Valid D4 parser tour returns zero errors.
- [ ] Parser summary reports a lossless round trip.
- [ ] CST tree contains function, binary, call, and control-flow nodes.
- [ ] Trivia tree contains documentation and block-comment tokens.
- [ ] Invalid block fixture emits `PAR003` and exit code 1.
- [ ] Invalid expression fixture emits `PAR005` and exit code 1.
- [ ] Parser JSON decodes successfully.

## Gate

D5 starts only after the user reports:

```text
GG D4 Passed
```
