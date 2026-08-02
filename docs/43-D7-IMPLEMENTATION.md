# D7 Implementation — Nominal Types and Members

D7 extends the D6 static checker from named type references to actual nominal
type bodies. The compiler now indexes fields, enum variants, inherent methods,
and trait implementation methods before checking function bodies.

## Implemented pipeline

1. Parse record construction as a lossless `record_expression`.
2. Index every local `record`, `struct`, and `enum`.
3. Parse declared field and enum payload types.
4. Collect inherent and trait implementation method signatures.
5. Replace `Self` inside method parameters and results with the implementation target.
6. Type-check record construction, field access, methods, enum variants, and mutation.
7. Expose the model through `nivra typecheck --nominals` and JSON output.

## Deliberate D7 boundary

D7 is still a single-source-file checker. Cross-module privacy, full generic
substitution, trait selection, pattern exhaustiveness, ownership flow, and HIR
lowering remain later deliveries.
