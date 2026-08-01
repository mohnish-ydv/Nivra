# Parser Diagnostics and Error Recovery

## Objective

A parser that stops at the first mistake creates developer pain. D4 instead emits
an error node, preserves source tokens, and searches for a safe synchronization
boundary so later declarations and statements can still be parsed.

## Diagnostic codes

- `PAR001` — unexpected syntax token
- `PAR002` — required syntax element missing
- `PAR003` — unclosed delimiter or construct
- `PAR004` — declaration expected
- `PAR005` — expression expected

Diagnostics use the D3 renderer, so human and JSON output share the same source
spans, codes, labels, notes, and help text.

## Recovery boundaries

Top-level recovery recognizes declaration starts, newlines, semicolons, closing
braces, and EOF. Statement recovery also recognizes common statement starts.
Field/variant recovery uses commas, newlines, closing braces, and EOF.

## No token deletion

Recovery advances over source tokens only by placing them in an `error` CST node.
This preserves formatter and IDE visibility and guarantees lossless round trips
for invalid files.

## Error limits

D4 has no global diagnostic cap because fixtures are small. A bounded diagnostic
budget will be added before untrusted large-source compilation to prevent error
storms and denial-of-service behavior.
