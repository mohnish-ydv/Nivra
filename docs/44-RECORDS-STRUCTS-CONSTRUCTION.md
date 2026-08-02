# Records, Structs, and Construction

Records and structs declare named fields:

```nva
record User {
    name: String
    age: Int = 0
}
```

Construction is explicit and named:

```nva
let user = User {
    name: "Mohnish",
}
```

The checker rejects missing required fields, unknown fields, duplicated
initializers, and incompatible values. Fields with defaults may be omitted.
Construction never relies on positional ordering.
