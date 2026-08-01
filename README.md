# Nivra — D6 Static Type-Checker Foundation (Build Fix)

> **Mission:** Power without the pain.

This is the corrected cumulative D6 archive. It repairs the GitHub Actions
`E0432` failure caused by an undeclared parser dependency in `nivra-types` tests
and adds a repository-wide Cargo dependency preflight so the same class of error
is detected before Rust compilation.

## Cumulative delivery status

- **D1:** mission, developer pain map, constitution, syntax direction
- **D2:** type/memory/error/concurrency architecture and Specification Draft 0.2
- **D3:** Rust workspace, source manager, diagnostics, lexer, first CLI
- **D4:** lossless CST parser, typed AST foundation, recovery, `nivra parse`
- **D5:** scopes, symbols, module indexing, name resolution, `nivra resolve`
- **D6:** static type representation, signatures, local inference, operator/call/
  condition/assignment/return validation, and `nivra typecheck`
- **D6 build fix:** declares the test-only `nivra-parser` edge in both
  `Cargo.toml` and `Cargo.lock`; validates all local imports and lock edges

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

## Verify on Android + Termux

```bash
bash scripts/termux-verify.sh
```

Expected final marker after actual Rust compilation and tests:

```text
★★★★★ D6 GOLDEN BUILD
```

Read `D6-BUILD-FIX-REPORT.md` for the exact root cause and
`MANUAL-VERIFICATION.md` for post-Actions checks.
