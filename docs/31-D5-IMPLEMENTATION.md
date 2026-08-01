# D5 Implementation — Semantic Index and Name Resolution

D5 is the first compiler delivery that answers what a name refers to. It adds the
`nivra-sema` crate and connects it to `nivra check` after successful parsing.

Implemented pipeline:

```text
source → lexer → lossless parser → semantic module index → lexical resolver
```

The pass indexes modules, imports, top-level declarations, extern functions,
methods, fields, enum variants, generic parameters, function parameters, locals,
loop patterns, match-arm patterns, closures, and task-group handles.

D5 deliberately does not infer or compare types. Member lookup such as
`value.method`, unknown type diagnostics, overload resolution, trait selection,
and ownership checking are later stages.
