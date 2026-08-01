# Name Resolution

Value lookup begins in the current lexical scope and walks parent scopes until a
matching symbol is found. Unresolved lowercase/value names emit `SEM003` with an
actionable suggestion when a nearby visible name exists.

Patterns introduce names in their correct region:

- `let` and `var` after their initializer
- `for` variables inside the loop body
- `if let` variables inside the success block
- match variables inside one arm
- closure parameters inside the closure body
- task-group handles inside the task-group body

Uppercase unresolved paths and member access are intentionally deferred to the D6
type checker so D5 does not invent incomplete type semantics.
