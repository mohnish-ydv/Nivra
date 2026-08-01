# D3 Compiler Foundation

D3 is the first implementation delivery. It converts the architecture agreed in
D2 into a buildable Rust workspace without pretending that a parser or compiler
backend already exists.

## Implemented compiler stages

| D2 stage | D3 status | Implementation |
|---|---|---|
| CP-01 Source manager | Implemented | `nivra-source` |
| CP-02 Unicode and line map | Implemented foundation | `nivra-source` |
| CP-03 Lexer | Implemented foundation | `nivra-lexer` |
| Driver foundation | Implemented | `nivra-cli` |
| CP-04 Parser onward | Not implemented | D4+ |

## Workspace principles

- zero third-party runtime dependencies
- safe Rust only
- UTF-8 source
- byte spans internally
- one-based Unicode scalar columns in diagnostics
- lossless trivia tokens
- deterministic human and JSON output
- no fixed `/tmp` verification files
- same tests in GitHub Actions and Termux

## Honest meaning of `nivra check` in D3

`nivra check` currently means:

1. read a UTF-8 source file
2. build its line map
3. tokenize it
4. report lexical errors

It does not yet parse grammar, resolve names, check types, validate ownership, or
generate code. A syntactically nonsensical sequence made only from valid tokens
may therefore pass D3 `check`. The CLI states this limitation in its help output.

## Version

D3 identifies the toolchain as `0.3.0`. This is an engineering version and not a
public language stability promise.
