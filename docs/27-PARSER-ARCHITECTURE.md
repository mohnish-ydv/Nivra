# Parser Architecture

## Strategy

D4 combines two techniques:

1. recursive descent for declarations, statements, delimiters, and control flow
2. Pratt parsing for prefix, postfix, binary, range, and assignment expressions

No parser generator or registry dependency is required. This keeps bootstrap
behavior auditable and Termux-friendly.

## Parsed declaration families

- modules and imports
- constants, type aliases, and newtypes
- records and layout-sensitive structs
- enums and payload variants
- traits and implementations
- ordinary, async, unsafe, and foreign functions
- generic parameter lists, parameters, return types, and where clauses
- C foreign blocks

## Parsed statement and expression families

- immutable and mutable bindings
- return, break, continue, defer, and ensure
- while and for
- if, if-let, else-if, and match
- task groups, async blocks, unsafe capability blocks
- try, await, spawn, closures, calls, members, indexing, and postfix try
- literals, names, tuples, arrays, unary operations, ranges, comparisons,
  boolean logic, arithmetic, shifts, and assignment

## Operator precedence

Assignments are right associative. Range, logical, bitwise, equality,
comparison, shift, additive, and multiplicative levels are ordered explicitly.
Postfix calls, members, indexing, and `?` bind more tightly than prefix and binary
operators.

## Generic close tokens

The parser treats `>>` as two generic closing angles while inside generic/type
contexts, without changing its lossless token spelling. Outside those contexts it
remains the shift-right operator.

## Deliberate D4 boundaries

- no semantic distinction between type and value names
- no name resolution or import graph
- no inferred types
- no ownership checking
- no macro expansion
- no incremental reparse algorithm yet
