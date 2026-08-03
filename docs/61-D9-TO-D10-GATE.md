# D9 to D10 Gate

D9 is ready to hand off only when all of the following pass from a fresh extraction:

- all D1–D9 structure linters;
- Cargo metadata with `--locked` and no registry dependencies;
- Rust formatting check;
- all workspace targets compile on Rust 1.74.0;
- all unit and integration tests pass with no fail-fast masking;
- focused D7 and D8 build-fix regressions pass;
- focused D9 parser, type, ownership, CLI, branch-join, scope-expiry, partial-move, and await regressions pass;
- every valid D9 fixture exits zero;
- every invalid fixture exits one and emits its pinned code;
- ownership JSON parses with Python;
- release binary smoke tests pass;
- the release archive survives a second clean extraction and verification.

D10 must build on the emitted ownership and exit plans. It must not move ownership rules back into type checking or introduce executable-backend claims without actual build and runtime verification.
