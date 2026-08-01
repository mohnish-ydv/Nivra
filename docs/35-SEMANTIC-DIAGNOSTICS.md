# Semantic Diagnostics

D5 adds six stable diagnostics:

- `SEM001` duplicate module-level name
- `SEM002` duplicate local binding
- `SEM003` unresolved value name
- `SEM004` multiple module declarations
- `SEM005` duplicate parameter or generic parameter
- `SEM006` duplicate field, variant, or method

Duplicate diagnostics label both declarations. Unresolved-name diagnostics label
the use and may suggest the closest visible spelling. Human and JSON renderers use
the same diagnostic objects.
