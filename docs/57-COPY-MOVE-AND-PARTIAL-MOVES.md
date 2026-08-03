# Copy, Move, and Partial Moves

## Structural Copy classification

Nivra does not make ownership dependent on spelling alone. D9 classifies a static type structurally:

- `Unit`, `Never`, `Bool`, `Char`, `Int`, `Float`, shared references, raw pointers, and function values are Copy.
- `String`, mutable references, owning containers, unknown types, and unconstrained generic parameters are Move.
- tuples are Copy only when every element is Copy.
- records, structs, and enums are Copy only when all stored fields and variant payloads are Copy after substituting concrete generic arguments.
- recursive or unresolved nominal cycles are conservatively Move.

A consuming context copies a Copy place and moves a Move place. Observer built-ins such as `print` read rather than consume.

## Explicit move

`move value` forces a consuming ownership use while preserving the operand's static type. This is useful when intent should be visible even outside a function call.

## Partial moves

Moving `user.name` does not immediately invalidate unrelated Copy fields, but it makes the complete `user` unavailable until the moved place is reinitialized. A mutable owner can recover:

```nva
var user = User(name: "M", age: 13)
let name = move user.name
user.name = "Nivra"
print(user)
```

## Flow joins

- moved on every incoming path -> moved;
- available on every incoming path -> available;
- different incoming states -> maybe moved;
- reinitialization restores availability for the assigned whole value or moved field.

Loops conservatively join zero iterations with one analyzed iteration. This is safe and intentionally more restrictive than future fixed-point dataflow.
