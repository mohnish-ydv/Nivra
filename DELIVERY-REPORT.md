# D3 Delivery Report

## Delivery

- Delivery: D3
- Builds on: D1 + D2
- Version: 0.3.0
- Status: IMPLEMENTED; cumulative static QA passed; compile acceptance is enforced by GitHub Actions and the Termux verifier
- Implementation language: Rust
- Third-party runtime dependencies: 0
- Verification targets: GitHub Actions, Linux, Android + Termux

## D2 defect fixed

The D2 verifier redirected output to the fixed path:

```text
/tmp/nivra-d1-lint.txt
```

That path returned `Permission denied` in the user's Termux environment. D3 removes
the fixed temporary file entirely. Specification linters now stream directly, and
all verification scratch work stays inside a disposable project-local directory.

## Implemented outcomes

1. Created a four-crate Rust workspace.
2. Implemented stable `SourceId`, checked byte spans, UTF-8 source loading, virtual
   sources, CRLF handling, and Unicode-aware line/column lookup.
3. Implemented structured diagnostics with codes, severities, labels, notes, help,
   deterministic phone-friendly rendering, and JSON output.
4. Implemented a lossless hand-written lexer retaining whitespace and comments.
5. Implemented all 45 D2 keywords.
6. Implemented Unicode identifiers with common combining-mark support.
7. Implemented nested block comments and documentation comments.
8. Implemented integer and floating-point literals, base validation, exponent
   validation, string/character escapes, and recovery diagnostics.
9. Added bidirectional-control and NUL-byte detection.
10. Implemented `nivra check`, `lex`, `explain`, `doctor`, help, and version.
11. Added 20 unit and CLI integration tests.
12. Added valid and invalid D3 fixtures.
13. Added cumulative D1/D2/D3 verification and GitHub Actions release artifact.
14. Added a Termux-safe verification wrapper that builds under `$HOME`.

## Honest boundaries

D3 does not include:

- parser or CST
- AST lowering
- name resolution
- type checking
- ownership checking
- interpreter
- C11 backend
- native Nivra executable generation
- package manager

A file containing valid tokens in invalid grammatical order may pass D3 `check`.
D4 will close that gap with parsing and syntax diagnostics.

## Next delivery

D4 target:

- lossless CST
- recursive-descent declaration parser
- Pratt expression parser
- error recovery
- syntax diagnostics
- AST lowering foundation
