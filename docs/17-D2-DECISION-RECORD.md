# D2 Decision Record

## Locked

- Nivra pre-1.0 engineering identity
- `.nva`, `nivra`, `nivra.toml`, `nivra.lock`, Edition 2026
- Apache-2.0 project license
- Rust bootstrap compiler
- hand-written lexer and recursive-descent/Pratt parser
- lossless CST, AST, HIR, and backend-neutral MIR
- C11 + Clang reference backend
- LLVM optimized backend after MIR stabilization
- static nominal types with local inference
- `Unit` and `Never`
- fixed-width numeric primitives and stable convenience aliases
- checked integer overflow in every standard profile
- explicit non-literal numeric conversions
- records, structs, enums, newtypes, aliases, traits
- monomorphized generics and explicit dynamic dispatch
- move semantics for non-copy values
- deterministic destruction
- `Box`, `Shared`, `Weak`
- local borrows without user-written lifetimes
- no borrowed fields and no borrow-across-await in Edition 2026
- no mandatory tracing GC
- named unsafe capabilities
- `Result` and `try` for recoverable failures
- aborting, non-catchable panic
- structured task groups and cooperative cancellation
- `Send`/`Sync` auto traits
- C ABI only for V1 foreign interoperability
- reproducible lockfiles and no arbitrary install scripts
- editions for compatibility

## Deferred beyond D2

- exact optimizer pass schedule
- stable native Nivra ABI
- package registry operator and domain
- public trademark/legal clearance
- allocator trait surface
- WebAssembly browser runtime
- embedded profile details
- reflection and compile-time metadata API
- safe binding generator implementation
- self-hosting schedule
- stable debugger protocol
- advanced borrowed data structures requiring explicit lifetimes

## Rejected for Edition 2026

- exact C++ clone
- C++ ABI compatibility promise
- ambient null
- unchecked integer overflow in release builds
- mandatory tracing GC
- explicit lifetime annotations in ordinary V1 code
- borrowed fields
- borrow crossing await
- catchable panic
- unchecked routine exceptions
- implicit detached tasks
- hidden global async runtime mutation
- arbitrary package install scripts
- safety semantics that differ between debug and release
