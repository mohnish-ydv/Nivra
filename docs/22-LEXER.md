# Lossless Lexer

`nivra-lexer` is a hand-written, single-pass lexer matching the D2 architecture.

## Implemented token families

- all 45 D2 reserved keywords
- Unicode identifiers
- decimal, binary, octal, and hexadecimal integers
- decimal floating-point literals with exponents
- strings and character literals
- common and Unicode escapes
- delimiters and longest-match operators
- whitespace and newline trivia
- line and documentation comments
- nested block and documentation comments
- explicit EOF token

## Lossless behavior

Whitespace and comments remain tokens. This enables the future CST, formatter,
documentation extraction, code actions, and source-preserving refactors.

## Recovery

The lexer reports an error and continues when practical. It does not panic on:

- unknown characters
- malformed numbers
- invalid escapes
- unterminated strings
- unterminated character literals
- unterminated nested comments
- NUL bytes

## Security

Bidirectional control characters emit `LEX009`. Outside literals and comments they
are errors; inside text they are warnings. This makes invisible source-order tricks
visible during review.

## D3 limitation

String interpolation is preserved inside one string token. Dedicated interpolation
token modes will be added with the parser/CST work when the exact grammar needs them.
