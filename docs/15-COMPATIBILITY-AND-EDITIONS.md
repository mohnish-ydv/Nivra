# Compatibility and Editions

## Edition model

Nivra separates language syntax/semantics editions from compiler release versions.
The first edition is `2026`.

- A project declares its edition in `nivra.toml`.
- Compiler updates do not silently change the meaning of an existing edition.
- New editions are opt-in and may introduce migrations.
- One compiler may support multiple editions.

## Versioning

The toolchain uses semantic versioning after the first public alpha. Before 1.0,
minor releases may change unstable APIs, but accepted edition rules still require
migration notes and conformance updates.

## Deprecation

- A stable feature is deprecated before removal.
- Diagnostics identify replacement syntax or API.
- `nivra fix --edition` performs machine-safe migrations.
- Public standard-library removals require a documented support window.

## Package compatibility

Packages declare version ranges and minimum edition/toolchain requirements.
The resolver reports which constraint caused a conflict and suggests the smallest
safe action rather than selecting an incompatible graph.

## ABI stability

Nivra's native ABI is not stable in Edition 2026. Cross-version binary boundaries
use the C ABI or explicit serialized protocols. A stable Nivra ABI is a separate
future proposal.

## Specification authority

Normative Edition 2026 semantics live in the specification and conformance tests.
Compiler behavior that contradicts them is a compiler defect, not an undocumented
language rule.
