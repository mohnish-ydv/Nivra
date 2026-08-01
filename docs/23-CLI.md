# D3 CLI

The binary is named `nivra`.

## Commands

### `nivra check <FILE>`

Loads and lexes one source file. Exit codes:

- `0` — no lexical errors
- `1` — lexical errors were found
- `2` — command usage or source loading failed

`--json` prints a machine-readable summary.

### `nivra lex <FILE>`

Prints the token stream. Trivia is hidden by default.

- `--trivia` includes whitespace and comments
- `--json` prints token objects

### `nivra explain <CODE>`

Explains D3 `LEX`, `CLI`, and `DRV` codes.

### `nivra doctor`

Reports host and D3 component status.

### `nivra version`

Reports toolchain version `0.3.0`.

## Commands deliberately absent

`run`, `build`, `test`, package operations, formatting, parsing, and code generation
are not exposed before their implementation exists.
