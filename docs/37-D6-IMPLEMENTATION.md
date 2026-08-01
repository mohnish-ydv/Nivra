# D6 Implementation

D6 adds the first static type-checking pass after the verified D5 semantic layer.
The implementation lives in the zero-dependency `nivra-types` crate and consumes
D4 CST nodes plus D5 name-resolution output.

## Pipeline

1. collect known primitive, imported, and nominal types
2. collect function and extern signatures before body checking
3. collect constant annotations
4. create lexical type environments for parameters and local bindings
5. infer expression and binding types
6. validate calls, operators, conditions, arrays, assignments, and returns
7. retain `Unknown` and `Error` recovery types to suppress cascades
8. expose typed reports through `nivra typecheck`

## Boundaries

D6 does not implement trait solving, member lookup, generic substitution, ownership
checking, exhaustiveness, HIR, MIR, or code generation. Member-heavy expressions
remain `Unknown` rather than producing speculative errors.
