# D8 Implementation: Generics and Trait Constraints

D8 extends the verified D7 lexer, parser, resolver, and type checker. It adds
first-class generic parameters to functions and nominal types, explicit and local
argument inference, non-generic traits, trait bounds, implementation validation,
and deterministic member selection.

The implementation remains an analysis-only compiler stage. It proves source
consistency and reports diagnostics; it does not yet emit HIR, MIR, C11, or native
code.

## Supported surface

- `fn identity<T>(value: T) -> T`
- `record Box<T> { value: T }`
- `enum Maybe<T> { empty, some(T) }`
- `impl<T> Box<T> { ... }`
- `T: Display` and `where T: Display`
- `impl Display for User`
- required and default trait methods
- explicit calls such as `identity<Int>(7)`
- inferred calls such as `identity(7)`

Generic traits, generic trait methods, specialization, higher-kinded types, and
code-generation monomorphization are explicitly deferred and diagnosed rather
than silently accepted.
