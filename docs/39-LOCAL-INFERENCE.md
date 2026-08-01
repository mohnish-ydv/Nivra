# Local Type Inference

D6 infers types from literals, arrays, operators, calls, names, blocks, conditions,
and annotated function signatures.

```nva
let count = 42          // Int
let ratio = 0.75        // Float
let title = "Nivra"    // String
let enabled = true      // Bool
let values = [1, 2, 3]  // List<Int>
```

Inference is deliberately local. It does not guess across module boundaries or
invent lossy conversions. `let missing = none` is rejected because `none` needs an
optional contextual type, while `let missing: String? = none` is valid.

Bindings are added after their initializer is checked, preserving D5's declaration
order rule. `let` is immutable and `var` is mutable.
