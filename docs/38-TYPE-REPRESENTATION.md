# Type Representation

The D6 `Type` model includes:

- recovery: `Unknown`, `Error`
- primitives: `Unit`, `Never`, `Bool`, `Char`, `String`, `Int`, `Float`
- nominal/generic: `Named(name, arguments)`
- absence: `Optional<T>` represented by source `T?`
- safe references: `&T`, `&mut T`
- FFI pointers: `*const T`, `*mut T`
- tuples
- function types

Integer width spellings normalize to the D6 integer family and floating width
spellings normalize to the floating family. Exact-width arithmetic and overflow
semantics remain governed by the D2 specification and later lowering passes.

`Unknown` means a later phase or unsupported D6 feature owns the answer. `Error`
means a primary type diagnostic has already been emitted. Both are compatible for
recovery only; neither is a source-level escape hatch.
