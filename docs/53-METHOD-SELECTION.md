# Deterministic Method Selection

Member lookup follows a stable order:

1. applicable inherent methods,
2. methods available through active generic constraints,
3. applicable trait implementations.

One inherent method wins over trait candidates. A single applicable trait method
is selected. Multiple equally applicable trait methods emit `TRT005`; the
compiler never chooses based on declaration order.

Receiver mutability remains enforced after generic substitution. An `&mut Self`
method cannot be called through an immutable place.
