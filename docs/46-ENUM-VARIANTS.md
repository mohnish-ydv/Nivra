# Enum Variants

D7 supports unit and tuple-payload enum variants:

```nva
enum State {
    idle
    ready(String)
    failed(String, Int)
}

let first = State.idle
let second = State.ready("done")
```

Variant payload arity and types are checked with `NOM007`. Record-payload variants
are parsed by the D4 grammar but their field-aware type model is deferred.
