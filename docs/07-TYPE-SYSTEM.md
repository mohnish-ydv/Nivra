# Type System — Edition 2026

## Character

Nivra uses static, mostly nominal typing with local inference. It favors explicit
public contracts, exact conversions, exhaustive modeling, and compile-time error
visibility.

## Primitive types

| Category | Types |
|---|---|
| no value | `Unit` with literal `()` |
| non-returning | `Never` |
| logic | `Bool` |
| signed integers | `I8`, `I16`, `I32`, `I64`, `I128`, `Isize` |
| unsigned integers | `U8`, `U16`, `U32`, `U64`, `U128`, `Usize` |
| floating point | `F32`, `F64` |
| text scalar | `Char` as a Unicode scalar value |
| owned text | `String` as validated UTF-8 |

Convenience names are stable aliases:

- `Int = I64`
- `UInt = U64`
- `Float = F64`
- `Byte = U8`

`Isize` and `Usize` match the target pointer width and are intended for address
and collection indexing boundaries, not ordinary domain values.

## Numeric behavior

- Unsuffixed integer literals use contextual typing and otherwise default to `Int`.
- Unsuffixed floating literals use contextual typing and otherwise default to `Float`.
- A literal may adapt to a target numeric type only when exactly representable.
- Runtime integer overflow traps in every build profile.
- Explicit wrapping and saturating operations are standard-library methods.
- Integer division by zero traps with a structured runtime diagnostic.
- Floating-point behavior follows IEEE 754 for `F32` and `F64`.
- Ordinary numeric conversions are explicit, even when widening.
- Fallible conversions return `Result`.

## Nominal data types

- `record` defines ordinary value-oriented data.
- `struct` defines representation-sensitive or explicitly mutable data.
- `enum` defines closed sum types and supports payload variants.
- `newtype` creates a distinct zero-overhead nominal type.
- `type` creates a transparent alias and no new identity.

```nva
newtype UserId = U64

type TimestampMillis = I64

record User {
    id: UserId
    name: String
}
```

## Bindings and inference

- `let` is immutable.
- `var` is mutable.
- Local variable types may be inferred from the initializer.
- Public functions, public fields, public constants, and public type members require
  explicit types.
- Inference never crosses a package boundary.
- The compiler does not infer public generic parameters from implementation details.

## Nullability and optionals

There is no `null`. `T?` is syntax sugar for `Option<T>`.

```nva
let email: String? = none
```

`Option<T>` is an enum with `some(T)` and `none`. Layout optimization is permitted
when it preserves observable semantics.

## Results

`Result<T, E>` is an enum with `ok(T)` and `err(E)`. Values of `Result` and types
marked `@must_use` cannot be silently discarded.

## Generics

- Generic functions and types use angle-bracket parameters.
- Constraints use traits.
- Compilation uses monomorphization for statically known types.
- Dynamic dispatch is explicit through `dyn Trait`.
- Generic specialization is absent in Edition 2026.
- Higher-kinded types are absent in Edition 2026.
- User-declared variance is absent in Edition 2026.

## Traits

Traits define behavior contracts without class inheritance.

- A type may implement multiple traits.
- Implementations obey an orphan rule: an implementation is legal when the trait
  or the target type belongs to the current package.
- Conflicting implementations are compile errors.
- Traits may contain required methods and methods with default bodies.
- Trait object use is explicit and restricted to object-safe traits.

## Copies, moves, and clones

- Primitive scalar types and eligible aggregates implement `Copy`.
- Non-`Copy` values move on assignment, argument passing, and return.
- Use after move is a compile error.
- Expensive duplication is explicit through `clone()` and the `Clone` trait.
- A type implementing `Drop` cannot implement `Copy`.

## Pattern matching

- Matches over closed enums and booleans are exhaustive.
- The compiler reports missing cases with suggested patterns.
- Guards are permitted but do not count as exhaustive coverage.
- A wildcard pattern is explicit and discouraged in public compatibility-sensitive
  enum handling.

## Function types and closures

Function types use `fn(A, B) -> R`. Closures use `|parameters| expression` or a
braced body. Capture mode is inferred as borrow, mutable borrow, or move; `move`
forces ownership capture.

## Conversions

- No general truthiness.
- No implicit string conversion.
- No implicit numeric conversion apart from exact literal adaptation.
- `as` is reserved for statically safe representation-preserving conversion.
- Potentially failing conversion uses `try_to_*` or the `TryFrom` trait.
- Formatting uses explicit `Display` or `Debug` capabilities.

## Type-system exclusions

Edition 2026 excludes class inheritance, ambient null, implicit narrowing,
unchecked union access, unrestricted operator overloading, and implicit public
API inference.
