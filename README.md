# Nivra — D5 Semantic Index & Name Resolution

> **Mission:** Power without the pain.

Nivra is a statically typed, compiled, general-purpose programming language and
integrated toolchain designed around safer defaults, actionable diagnostics, and a
single official workflow.

## Delivery status

- **D1:** mission, pain map, constitution, syntax direction
- **D2:** language architecture and Specification Draft 0.2
- **D3:** Rust compiler workspace, source manager, diagnostics, lexer, CLI
- **D4:** lossless CST parser, Pratt expressions, recovery, AST foundation
- **D5:** semantic AST accessors, module index, scopes, symbol tables, name resolution

## D5 operational commands

```bash
nivra check file.nva
nivra resolve file.nva
nivra resolve file.nva --symbols --scopes
nivra resolve file.nva --json
nivra explain SEM003
nivra doctor
```

`nivra check` now runs source loading, lexing, parsing, and semantic name
resolution. Type checking is deliberately not claimed in D5.

## Workspace

D5 contains seven local-only Rust crates:

- `nivra-source`
- `nivra-diagnostics`
- `nivra-lexer`
- `nivra-syntax`
- `nivra-parser`
- `nivra-sema`
- `nivra-cli`

There are no third-party runtime crate dependencies.

## Verify

```bash
bash scripts/termux-verify.sh
```

Expected final line:

```text
★★★★★ D5 GOLDEN BUILD
```

See `MANUAL-VERIFICATION.md` for the exact checks after GitHub Actions is green.
