# Borrow Region Inference

Edition 2026 has no user-written lifetime parameters. D9 infers local borrow regions from owner scope and reference use.

## Rules

- `&T` creates a shared borrow. Overlapping shared borrows may coexist.
- `&mut T` creates an exclusive mutable borrow and requires a `var` owner.
- a mutable borrow conflicts with any overlapping live borrow;
- a shared borrow conflicts with an overlapping live mutable borrow;
- moving or assigning an overlapping place is rejected while borrowed;
- direct owner access is rejected while an exclusive borrow is live.

## Last-use regions

For `let view = &owner`, the loan extends through the last use of `view`, not automatically through the entire function. This allows sequential borrowing without lifetime annotations:

```nva
let reader = &document
print(reader)
let writer = &mut document
print(writer)
```

The analyzer also terminates loans created inside an inner lexical scope when that scope exits, even when the owner belongs to an outer scope. Borrows captured by `defer` are intentionally retained until scope exit because the deferred expression executes there.

## Edition 2026 restrictions

Borrowed record fields and enum payloads are rejected. A borrowed return must have exactly one borrowed input origin. A local borrow cannot escape a function, including through a local reference alias or tail expression. A live borrow cannot cross `await`. These restrictions preserve predictable compilation while the language avoids explicit lifetime syntax.
