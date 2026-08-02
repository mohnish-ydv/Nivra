# D8 to D9 Gate

D9 must not start until:

- the GitHub workflow `Verify D8 Generics and Traits` is green,
- all cumulative workspace tests pass,
- the complete D8 tour reports zero errors,
- every `GEN001`–`GEN006` and `TRT001`–`TRT006` fixture emits its code,
- human and JSON trait reports are manually inspected,
- the user reports `GG D8 Passed`.

D9 is reserved for ownership-flow foundations: moves, copy classification,
use-after-move diagnostics, mutable-borrow exclusivity, and deterministic drop
planning. It must build on D8 substitutions rather than bypassing them.
