# D8 Diagnostics

## Generic diagnostics

- `GEN001` wrong generic arity
- `GEN002` generic argument cannot be inferred
- `GEN003` conflicting inference
- `GEN004` unsatisfied trait bound
- `GEN005` invalid or duplicate generic declaration
- `GEN006` generic trait feature is explicitly deferred

## Trait diagnostics

- `TRT001` unknown trait
- `TRT002` conflicting implementation
- `TRT003` missing required method
- `TRT004` implementation signature mismatch
- `TRT005` ambiguous method selection
- `TRT006` package orphan-rule violation

Every code is available through `nivra explain CODE`, human diagnostics, JSON
output, dedicated fixtures, and automated smoke tests.
