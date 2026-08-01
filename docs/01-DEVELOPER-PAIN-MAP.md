# Developer Pain Map

The pain map is the product backlog for the language itself. Priority meanings:

- **P0:** the language loses its reason to exist if this remains unsolved
- **P1:** required for a serious production ecosystem
- **P2:** valuable after the foundation is stable

## P0 — Identity-defining problems

| ID | Problem | Root cause | Trion direction | Measurable outcome |
|---|---|---|---|---|
| DP-001 | Build-system fragmentation | Compilers, generators and package tools use separate configuration models | One official project model and CLI | Common projects require no handwritten build script |
| DP-002 | Dependency version conflicts | Global state and weak lock semantics | Isolated dependency graph and mandatory lockfile for applications | Same commit resolves identical versions |
| DP-003 | Hostile diagnostics | Errors expose compiler internals instead of developer intent | Structured diagnostics with cause, context and fix guidance | Primary error fits on one phone screen where possible |
| DP-004 | Null dereference | Null is ambient and unchecked | Non-null types by default; explicit `T?` | Safe code cannot dereference absence |
| DP-005 | Memory lifetime bugs | Ownership is implicit or manually coordinated | Safe lifetime model with explicit low-level escape hatch | No use-after-free/double-free in safe code |
| DP-006 | Unhandled recoverable errors | Exceptions or status codes escape review | Typed `Result` values and compiler-visible propagation | Public fallible calls cannot be silently ignored |
| DP-007 | Data races | Shared mutation is easy and thread ownership is unclear | Sendability checks and structured concurrency | Safe concurrent code is race-resistant by construction |
| DP-008 | Tool fragmentation | Formatter, linter, tester and docs tools disagree | Official integrated toolchain | One install provides core workflow |
| DP-009 | Irreproducible environments | Hidden SDK/tool versions and mutable registries | Toolchain manifest, lockfile, content hashes | CI and phone builds use the same declared inputs |
| DP-010 | Boilerplate-heavy data models | Constructors, equality, debug and serialization are repeated | `record` types with derived capabilities | Common immutable model needs only field declarations |
| DP-011 | Dangerous implicit conversions | Narrowing and truthiness hide defects | Explicit conversions; no general truthiness | Lossy conversion requires visible code |
| DP-012 | Orphaned async work | Detached tasks outlive their owner | Task scopes with cancellation propagation | Scoped tasks finish or cancel before scope exit |

## P1 — Production ecosystem problems

| ID | Problem | Root cause | Trion direction | Measurable outcome |
|---|---|---|---|---|
| DP-013 | Slow incremental builds | Excessive global recompilation | Module-level fingerprints and cacheable artifacts | Small edits avoid full rebuild |
| DP-014 | Configuration sprawl | Every tool invents a config file | One project manifest; convention-first defaults | Typical project has one authored config file |
| DP-015 | Cross-compilation complexity | Platform setup leaks into source | Explicit target profiles and capability checks | Target errors identify the exact missing component |
| DP-016 | Unsafe foreign-function boundaries | ABI assumptions are implicit | Declared FFI blocks and generated bindings metadata | Unsafe boundary is searchable and auditable |
| DP-017 | Supply-chain risk | Packages execute install scripts or change content | Signed metadata, hashes and no arbitrary install scripts by default | Locked package content is tamper-evident |
| DP-018 | Refactoring uncertainty | Text edits bypass semantic meaning | Language server is part of the official toolchain | Rename and references use compiler semantics |
| DP-019 | Test setup overhead | Frameworks and runners are external | Built-in test syntax and runner | `trion test` works in a new project |
| DP-020 | Documentation drift | API docs are detached from type checking | Docs generated from checked source and examples | Documentation examples compile in CI |
| DP-021 | Hidden performance costs | Allocation/copy/runtime behavior is invisible | Explicit cost model and opt-in diagnostics | Tooling can explain allocations and copies |
| DP-022 | Secret leakage | Credentials enter source or logs | Standard secret APIs and redaction metadata | Debug formatting redacts marked secrets |
| DP-023 | Serialization schema drift | Wire names and compatibility are ad hoc | Version-aware derive system with explicit changes | Breaking schema changes produce diagnostics |
| DP-024 | Platform capability ambiguity | APIs fail only after deployment | Capability declarations and target validation | Unsupported APIs fail at build time when knowable |
| DP-025 | Backward compatibility anxiety | Releases change behavior without policy | Edition-based evolution and compatibility guarantees | Existing edition behavior remains stable |

## P2 — Expansion problems

| ID | Problem | Root cause | Trion direction | Measurable outcome |
|---|---|---|---|---|
| DP-026 | Debugging without context | Stack traces omit domain values and async lineage | Structured stack traces and task lineage | Failure report identifies task and call path |
| DP-027 | Profiling setup friction | Profilers are platform-specific | Official profiling command and portable event format | `trion profile` works without project edits |
| DP-028 | Large-repository navigation | Module boundaries and ownership are unclear | Workspace graph and dependency visualization | Cycles and hotspots are reportable |
| DP-029 | Unsafe code review difficulty | Escape hatches spread through normal code | Named unsafe capabilities and audit reports | Toolchain lists every unsafe site |
| DP-030 | Onboarding inconsistency | Tutorials and templates diverge from current releases | Versioned official templates and executable guides | Fresh template passes current CI unchanged |

## P0 release gate

A public V1 cannot be called successful if any P0 item is merely documented but
not enforced by language semantics or official tooling.
