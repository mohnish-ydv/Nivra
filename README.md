# Nivra — D3 Compiler Foundation

> **Mission:** Power without the pain.

Nivra is a statically typed, compiled, general-purpose programming language and
integrated toolchain designed to remove recurring developer headaches while
preserving native performance and low-level control.

D3 is the first executable implementation delivery.

## Delivery progression

- **D1:** mission, developer pain map, constitution, syntax direction
- **D2:** identity, type system, memory model, error model, concurrency,
  compiler architecture, backend, ABI/FFI, package model, grammar, compatibility
- **D3:** Rust workspace, source manager, Unicode line map, structured diagnostics,
  lossless lexer, and first operational `nivra` CLI

## Implemented commands

```bash
nivra check examples/d3/01_hello.nva
nivra lex examples/d3/01_hello.nva
nivra lex examples/d3/02_unicode_and_comments.nva --trivia
nivra explain LEX005
nivra doctor
nivra --version
```

`nivra check` in D3 performs source loading and lexical checking only. Parsing,
type checking, ownership validation, execution, and native code generation are
not claimed yet.

## Rust workspace

- `nivra-source` — files, source IDs, spans, Unicode-aware line maps
- `nivra-diagnostics` — actionable human and JSON diagnostics
- `nivra-lexer` — lossless hand-written lexer with recovery
- `nivra-cli` — initial compiler driver

The workspace has zero third-party runtime dependencies. Detailed preflight evidence is in `D3-QA-REPORT.md`.

## Verify

On Linux or a Termux-internal filesystem:

```bash
bash verify.sh
```

Expected ending:

```text
★★★★★ D3 GOLDEN BUILD
```

For an archive extracted in Android Downloads, use:

```bash
bash scripts/termux-verify.sh
```

That script copies the repository to Termux home before compiling, avoiding
Android shared-storage executable restrictions.

## Current identity

- Language: `Nivra`
- Command: `nivra`
- Source extension: `.nva`
- Edition: `2026`
- Compiler foundation version: `0.3.0`
- Bootstrap implementation: Rust
- License: Apache-2.0
