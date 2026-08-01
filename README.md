# Nivra — D4 Parser & AST Foundation

> **Mission:** Power without the pain.

Nivra is a statically typed, compiled, general-purpose programming language and
integrated toolchain designed to remove recurring developer headaches while
preserving native performance and low-level control.

## Delivery status

- **D1:** mission, pain map, constitution, syntax direction
- **D2:** identity, type/memory/error/concurrency architecture and spec draft
- **D3:** Rust workspace, source manager, diagnostics, lexer, initial CLI
- **D4:** lossless CST parser, Pratt expressions, recovery, typed AST foundation

## What works in D4

```bash
nivra check file.nva
nivra lex file.nva --trivia
nivra parse file.nva
nivra parse file.nva --tree
nivra parse file.nva --tree --trivia
nivra parse file.nva --json
nivra explain PAR003
nivra doctor
```

`nivra check` now performs lexical and syntax validation. D4 does not yet resolve
names, check types, generate native code, or execute programs.

## Workspace

- `nivra-source` — source files, IDs, spans, Unicode positions
- `nivra-diagnostics` — human and JSON diagnostics
- `nivra-lexer` — lossless Edition 2026 lexer
- `nivra-syntax` — immutable CST and typed AST wrappers
- `nivra-parser` — recursive descent + Pratt parser and recovery
- `nivra-cli` — operational compiler driver

All compiler crates use only local path dependencies in D4.

## Verify

On GitHub Actions or a Rust-enabled machine:

```bash
bash verify.sh
```

On Android + Termux, use the internal-storage-safe wrapper:

```bash
bash scripts/termux-verify.sh
```

Expected final line:

```text
★★★★★ D4 GOLDEN BUILD
```

See `MANUAL-VERIFICATION.md` for the exact checks to run after Actions turns green.
