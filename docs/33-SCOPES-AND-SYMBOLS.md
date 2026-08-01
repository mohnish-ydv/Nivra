# Scopes and Symbol Tables

D5 creates an explicit parent-linked scope tree. The prelude is the root, the file
module is its child, and functions, blocks, loops, match arms, closures, task
groups, and type bodies create nested scopes where required.

Module declarations are indexed before function bodies, allowing functions to call
other module functions regardless of source order. Local bindings become visible
only after their initializer, preventing accidental self-reference and exposing
use-before-declaration errors.

Imports create local module bindings. Public visibility is recorded in the index;
cross-module privacy enforcement will become authoritative when multi-file package
loading is implemented.
