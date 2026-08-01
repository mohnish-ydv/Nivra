# Operators, Calls, Conditions, and Returns

## Operators

- arithmetic requires equal numeric families
- string concatenation accepts `String + String`
- logical operators require `Bool`
- bitwise and shift operators require `Int`
- comparisons require compatible operands
- implicit `Int` to `Float` conversion is rejected

## Calls

D6 indexes signatures before checking bodies, so functions may call functions
written later in the file. Calls validate argument count and parameter types.
Member calls remain recoverable until D7 adds richer nominal/member typing.

## Conditions

`if`, `while`, `ensure`, and `assert` conditions require `Bool`. Nivra has no
truthiness.

## Returns

Explicit `return` expressions and expression-valued function bodies are checked
against the declared return type. Unit functions reject an accidental non-Unit
tail value.
