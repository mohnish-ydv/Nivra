# Type Diagnostics

D6 reserves `TYP001` through `TYP010`:

| Code | Meaning |
|---|---|
| TYP001 | assignment or initializer type mismatch |
| TYP002 | unsupported operator or typed operation |
| TYP003 | wrong function argument count |
| TYP004 | function argument type mismatch |
| TYP005 | function return mismatch |
| TYP006 | insufficient inference context |
| TYP007 | non-Boolean condition |
| TYP008 | unknown or malformed declared type |
| TYP009 | heterogeneous array elements |
| TYP010 | assignment to immutable `let` |

Diagnostics include a primary source location, expected/found types where relevant,
and one actionable correction. Recovery types prevent duplicate noise after the
primary error.
