# Deterministic Drop Planning

D9 generates a stable scope-exit plan suitable for later HIR/MIR lowering.

For each lexical scope:

1. registered `defer` actions execute in reverse registration order;
2. remaining locals whose concrete type needs destruction drop in reverse declaration order.

Fully moved values do not receive a drop action. Maybe-moved or partially moved values receive a conditional drop-plan entry, representing a future drop flag. Drop necessity is computed separately from Copy/Move classification: references and raw pointers never receive drop glue, including move-only mutable references.

The plan is exposed through:

```text
nivra ownership FILE --drops
nivra ownership FILE --json
```

D9 does not claim executable destruction yet. It produces deterministic compiler data that the future lowering/backend phases can consume without redesigning ownership semantics.
