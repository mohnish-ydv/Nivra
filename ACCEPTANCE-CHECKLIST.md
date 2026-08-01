# D3 Acceptance Checklist

## D2 repair

- [x] Fixed `/tmp/nivra-d1-lint.txt` permission failure.
- [x] No fixed temporary output path remains in verification scripts.
- [x] D1 regression checks pass.
- [x] D2 architecture and grammar checks pass.

## Implementation

- [x] Four Rust crates exist and use workspace metadata.
- [x] Runtime dependency count is zero.
- [x] Source IDs, spans, source loading, and line maps are implemented.
- [x] Human and JSON diagnostics are implemented.
- [x] Lossless trivia tokenization is implemented.
- [x] All 45 D2 keywords are implemented.
- [x] Nested comments, Unicode identifiers, numbers, strings, and chars are tested.
- [x] Lexer errors recover without compiler panic.
- [x] `nivra check`, `lex`, `explain`, `doctor`, and version are implemented.
- [x] Valid and invalid examples are included.
- [x] GitHub Actions builds and uploads the release binary.

## Manual after GitHub Actions is green

- [ ] `bash scripts/termux-verify.sh` prints the D3 golden marker.
- [ ] `nivra --version` reports `0.3.0`.
- [ ] valid example reports zero lexical errors.
- [ ] invalid string reports `LEX002` and exit code 1.
- [ ] `nivra lex --trivia` shows comments and whitespace.
- [ ] `nivra check --json` prints valid JSON.
- [ ] user reports `GG D3 Passed`.
