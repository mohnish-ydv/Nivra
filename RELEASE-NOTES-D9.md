# Nivra 0.9.0 — D9 Ownership and Borrow Foundation

D9 adds Nivra's first flow-sensitive ownership phase while preserving the D1–D8 compiler architecture. It runs only after successful semantic and type analysis and emits ownership events, diagnostics, binding states, and a deterministic scope-exit plan for future lowering.

## User-visible language behavior

Values are structurally classified as `Copy` or `Move`. Moving a move-only value invalidates the source, while copyable values remain available. `move expression` makes transfer intent explicit. Fields may move independently and can be reinitialized through a mutable owner.

Edition 2026 borrows use `&value` and `&mut value` without user-written lifetime parameters. Local reference regions end after their last use, mutable borrows require `var`, overlapping conflicting borrows are rejected, local borrows cannot escape, and active borrows cannot cross `await`.

## Compiler and tooling

The new `nivra ownership` command provides binding, event, and drop reports plus machine-readable JSON. `nivra check` includes ownership diagnostics. Every D9 diagnostic is supported by `nivra explain` and has a dedicated invalid fixture.

## Scope-exit plan

For each scope, deferred actions execute in reverse registration order, followed by drop-requiring locals in reverse declaration order. Fully moved values are omitted; partially or conditionally moved values carry a conditional drop flag. D9 plans these actions but does not yet emit executable destruction code.

## Compatibility

The language remains Edition 2026, source extension `.nva`, CLI `nivra`, manifest `nivra.toml`, lockfile `nivra.lock`, and reference backend C11 + Clang. The workspace has no third-party Rust dependencies and remains pinned to Rust 1.74.

## Verification status

All non-Rust artifact checks and fresh-extraction static checks pass in the delivery environment. Rust/Cargo was unavailable there, so compilation and test success are not represented as completed. The included GitHub Actions and Termux scripts are the authoritative executable verification path.
