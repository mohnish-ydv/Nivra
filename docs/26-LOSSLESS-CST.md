# Lossless Concrete Syntax Tree

## Why lossless

A compiler can discard comments and whitespace, but a formatter, refactoring
engine, IDE, documentation tool, and automated fix system cannot. Nivra therefore
retains every lexer token, including trivia and the final EOF token.

For a successfully loaded UTF-8 source file, D4 enforces this invariant:

```text
root.lossless_text(source) == source.text()
```

The invariant also holds for malformed syntax because recovery wraps unexpected
source tokens in `error` nodes instead of deleting them.

## Tree model

`SyntaxNode` contains:

- a stable `SyntaxKind`
- one covered byte span
- an ordered list of child nodes and tokens

`SyntaxToken` keeps the original D3 lexer token and source span. Token text is read
from the source manager, avoiding duplicate source strings inside the tree.

## Debug and JSON views

Human tree output is indentation based and stable enough for snapshot tests.
JSON output records node/token kinds, byte ranges, token text, parser recoveries,
and diagnostics. Trivia is omitted by default from displayed trees but remains in
the actual CST; `--trivia` exposes it.

## Immutability

D4 trees are immutable owned values. Future incremental parsing may replace the
storage layer, but the public invariants—ordered lossless children, stable kinds,
and source spans—must remain intact.
