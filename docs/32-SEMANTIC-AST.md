# Semantic AST Boundary

The CST remains lossless and immutable. D5 expands typed AST accessors and builds a
separate semantic index rather than mutating syntax nodes.

The semantic layer stores stable `SymbolId` and `ScopeId` values, source spans,
visibility, namespace, origin, and symbol category. This keeps diagnostics tied to
original source while allowing later HIR lowering to use compact identifiers.

Type and value namespaces are separate. A record named `Item` and a function named
`Item` may coexist, while two functions named `Item` in the same module conflict.
