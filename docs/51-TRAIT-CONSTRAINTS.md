# Trait Constraints

D8 supports non-generic traits with required or default methods. Bounds may be
written directly on a generic parameter or in a `where` clause.

```nva
fn render<T: Display>(value: T) -> String { value.display() }
fn render<T>(value: T) -> String where T: Display { value.display() }
```

Inside a constrained body, method lookup may use methods declared by the bound.
Inside a default trait method, `Self` is treated as satisfying the current trait,
so default behavior can call required methods safely.

Generic trait declarations and generic trait methods are rejected with `GEN006`
until their variance, substitution, and coherence semantics are specified.
