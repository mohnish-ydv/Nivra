# D9 Ownership Diagnostics

## Ownership

- `OWN001` — use or borrow after move.
- `OWN002` — move while an overlapping borrow remains live.
- `OWN006` — use of a moved field or complete value after a partial move.
- `OWN007` — value may be moved on one incoming control-flow path.

## Borrowing

- `BOR001` — mutable borrow conflicts with a live shared or mutable borrow.
- `BOR002` — shared borrow conflicts with a live mutable borrow.
- `BOR003` — mutable borrow requested from an immutable `let` owner.
- `BOR004` — assignment overlaps a live borrow.
- `BOR005` — direct owner access during a live mutable borrow.
- `BOR006` — borrowed field or enum payload stored in a nominal type.
- `BOR007` — borrowed return has no single unambiguous borrowed input.
- `BOR008` — borrow of a local value escapes the function.
- `BOR009` — live borrow crosses `await`.

Every code has `nivra explain CODE` support and a dedicated invalid D9 fixture. Diagnostics include primary spans, relevant declaration/borrow secondary spans, and actionable help.
