# Typed AST Foundation

The CST represents exact syntax. Semantic compiler stages need safer views that
cannot accidentally treat an enum declaration as a function. D4 introduces an
`AstNode` trait and zero-copy wrappers for major node classes.

Initial wrappers include:

- source files
- functions
- records
- enums
- traits
- implementations
- blocks
- let statements
- expression statements

A wrapper casts only when the underlying `SyntaxKind` matches. The wrapper stores
a reference to the existing CST node; it does not copy tokens or source text.

D5 will expand wrappers with named accessors such as function names, parameter
iterators, declared types, bodies, and declaration visibility. Semantic IDs and
resolved symbols will not be stored in the lossless CST; they belong in separate
compiler tables keyed by syntax identities/spans.
