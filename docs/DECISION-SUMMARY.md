# D1 Decision Summary

## Locked in D1

- The project is a statically typed, compiled, general-purpose language plus an
  official integrated toolchain.
- The primary product goal is reducing recurring developer pain.
- Native performance and low-level access remain goals.
- Safety is the default; unsafe capability is explicit and auditable.
- Bindings are immutable by default: `let` versus `var`.
- Types are non-null by default; absence is explicit.
- Routine recoverable errors use typed `Result` values.
- Concurrency follows a structured model.
- Braces delimit blocks; indentation is not semantic.
- Semicolons are optional at line endings.
- Blocks may produce values.
- Conditions require `Bool`; there is no general truthiness.
- Ordinary data uses `record`; layout-sensitive data uses `struct`.
- Behavior composition uses traits and implementations.
- Class inheritance is not a V1 feature.
- Public visibility is explicit.
- Lossy conversion is explicit.
- One official CLI owns the common workflow.
- Reproducible dependency locking is mandatory for applications.
- Diagnostics are a first-class product surface.
- Phone + Termux verification is a permanent project requirement.
- Every permanent feature must map to a developer pain ID.

## Deferred to D2

- final public language name and trademark/collision checks
- final source extension and executable command
- exact integer families, overflow mode, and numeric literal inference
- exact type inference boundaries
- exact memory model: ownership, regions, ARC, tracing, or hybrid
- move/copy semantics
- reference and borrowing syntax
- deterministic destruction rules
- exact `defer` semantics
- concurrency scheduler and cancellation semantics
- thread-safety/sendability rules
- async error aggregation
- native backend: C, LLVM, Cranelift, custom IR, or staged combination
- compiler implementation language
- stable ABI and FFI rules
- package registry protocol and trust model
- optional chaining and unwrap syntax
- generic variance and specialization
- derive/metaprogramming mechanism
- edition and compatibility cadence

## Rejected for V1

- exact C++ clone
- C/C++ source compatibility as a design constraint
- preprocessor
- header files
- multiple inheritance
- ambient null
- implicit narrowing conversions
- unrestricted operator overloading
- unchecked exceptions for ordinary failure
- multiple official build systems
- arbitrary dependency install scripts
- unrestricted textual macros
- unsafe behavior outside an explicit boundary
- hidden detached tasks
- language features without an identified developer pain

## Working identifiers

The working project name is `Trion`, the working command is `trion`, and the
working source extension is `.trn`. They are intentionally replaceable until D2
performs public-name due diligence. Their use in D1 examples does not make them a
permanent compatibility promise.
