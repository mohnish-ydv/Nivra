# Diagnostics Engine

The `nivra-diagnostics` crate treats compiler errors as a product surface.

## Diagnostic structure

Each diagnostic can carry:

- stable code
- severity
- main message
- primary source label
- secondary source labels
- notes
- actionable help

## Outputs

### Human

Designed for narrow Termux and GitHub Actions logs:

```text
error[LEX002]: unterminated string literal
 --> example.nva:2:15
  |
2 | let value = "open
  |             ^^^^^ string starts here
  = help: close the string with `"` before the line ends
```

### JSON

Machine-readable output includes code, severity, message, labels, byte ranges,
path, line, column, notes, and help. It is intended for the future language server,
NovaIDE integration, and external tooling.

## Stability rule

Diagnostic wording may improve before 1.0. Codes are the stable automation handle
within a development edition.
