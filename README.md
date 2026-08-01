# Nivra — D6 Static Type-Checker Foundation

> **Mission:** Power without the pain.

Nivra is a statically typed, compiled, general-purpose programming language and
integrated toolchain designed to remove recurring developer headaches while
preserving native performance and low-level control.

## Cumulative delivery status

- **D1:** mission, developer pain map, constitution, syntax direction
- **D2:** type/memory/error/concurrency architecture and Specification Draft 0.2
- **D3:** Rust workspace, source manager, diagnostics, lexer, first CLI
- **D4:** lossless CST parser, typed AST foundation, recovery, `nivra parse`
- **D5:** scopes, symbols, module indexing, name resolution, `nivra resolve`
- **D6:** static type representation, signatures, local inference, operator/call/
  condition/assignment/return validation, and `nivra typecheck`

## D6 operational commands

```bash
nivra check file.nva
nivra typecheck file.nva
nivra typecheck file.nva --functions --types
nivra typecheck file.nva --json
nivra resolve file.nva --symbols --scopes
nivra parse file.nva --tree
nivra lex file.nva --trivia
nivra explain TYP004
nivra doctor
```

## Workspace

D6 contains eight local-only Rust crates:

- `nivra-source`
- `nivra-diagnostics`
- `nivra-lexer`
- `nivra-syntax`
- `nivra-parser`
- `nivra-sema`
- `nivra-types`
- `nivra-cli`

There are no third-party Rust runtime dependencies.

## Verify

On Android + Termux:

```bash
bash scripts/termux-verify.sh
```

Expected final marker:

```text
★★★★★ D6 GOLDEN BUILD
```

See `MANUAL-VERIFICATION.md` for the exact checks after GitHub Actions turns green.
