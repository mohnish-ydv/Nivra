# Language Constitution

The constitution contains non-negotiable design rules. Syntax and implementation
may evolve; these principles require an explicit constitutional amendment.

## Article C-001 — Developer intent over ceremony

Common operations must be concise without hiding important behavior. Boilerplate
is a tooling defect unless it communicates a real semantic choice.

## Article C-002 — Safe by default

Ordinary code must exclude null dereference, use-after-free, double-free, invalid
aliasing, and unchecked data races. Unsafe operations require a visible boundary.

## Article C-003 — Power remains available

Safety must not prohibit systems work. Low-level memory, FFI, layout control, and
platform instructions may exist behind explicit, auditable capabilities.

## Article C-004 — Static meaning before execution

Types, effects, exhaustiveness, visibility, and recoverable-error obligations
should be checked before a program runs whenever practical.

## Article C-005 — No ambient null

Every ordinary value exists. Absence is represented explicitly by `T?`, `Option<T>`,
or a domain-specific type.

## Article C-006 — Recoverable errors are values

Routine failure uses typed results, not unchecked exceptions. Fatal defects may
terminate with a structured crash report.

## Article C-007 — Mutation is visible

Bindings are immutable by default. Mutation uses `var` and shared mutation requires
an explicit synchronization or ownership mechanism.

## Article C-008 — Concurrency is structured

Child tasks belong to a scope. Cancellation and failure propagate predictably.
Detached execution is an advanced, visibly named operation.

## Article C-009 — Performance is explainable

The language must publish a cost model. Hidden allocation, copying, synchronization,
or blocking behavior should be inspectable through tooling.

## Article C-010 — One official workflow

The official toolchain owns project creation, dependency resolution, build, run,
test, format, lint, documentation, packaging, and core language-server behavior.

## Article C-011 — Diagnostics are a product surface

A diagnostic must identify the primary location, explain the violated rule, avoid
compiler-internal noise, and provide a safe next action when one is known.

## Article C-012 — Reproducibility is the default

Application builds use locked dependencies and declared toolchain inputs. Network
access is not required after all locked artifacts are cached.

## Article C-013 — Readability defeats cleverness

Local code should reveal control flow, mutation, fallibility, unsafe operations,
and concurrency. Features that reward obscurity are rejected.

## Article C-014 — Compatibility is planned

Language evolution uses editions or another explicit compatibility mechanism.
Stable code does not silently change meaning under a toolchain update.

## Article C-015 — Tooling is part of the language

Formatting rules, semantic navigation, testing conventions, package metadata,
and documentation behavior are specified rather than left to incompatible tools.

## Article C-016 — Secure supply chain

Packages are content-addressed or hash-verified. Arbitrary dependency install
scripts are prohibited by default. Privileged build actions require consent.

## Article C-017 — No feature without a pain case

Every permanent feature must map to a documented developer pain, demonstrate why
a library/tool cannot solve it adequately, and state its complexity cost.

## Article C-018 — Phone-first participation

Core repository verification, documentation work, examples, issue triage, and
supported bootstrap workflows must remain usable from Android + Termux without
requiring a paid service or desktop-only IDE.

## Amendment rule

A constitution article can change only through:

1. a written problem statement
2. affected pain-map IDs
3. at least two considered alternatives
4. compatibility analysis
5. migration strategy
6. updated machine-readable constitution data
7. passing specification verification
