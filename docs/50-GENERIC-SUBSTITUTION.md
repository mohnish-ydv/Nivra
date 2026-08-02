# Generic Substitution

Each declaration owns an ordered generic-parameter list. Call checking creates a
substitution map from parameter names to concrete types. Explicit arguments seed
the map; argument/parameter unification fills missing entries.

A substitution is valid only when:

1. its arity matches the declaration,
2. one parameter is not inferred as conflicting concrete types,
3. every required parameter becomes concrete,
4. every declared trait bound is satisfied.

The checker recursively substitutes through optionals, references, pointers,
tuples, function types, and nested named types. `Unknown` remains a recovery type
only; it never counts as a successful generic proof.
