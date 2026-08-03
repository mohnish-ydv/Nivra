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

## D9 build-fix revision

The first uploaded D9 GitHub Actions run compiled the workspace and passed cumulative D7/D8 plus initial D9 regressions, then exposed call-style record construction in several D9-only fixtures. Those fixtures now use the established D7 brace syntax. The two warnings and workflow deprecations reported by that run are removed, warnings are fatal, formatting is non-mutating, missing `rustfmt` cannot produce a golden result, and the complete `--no-fail-fast` suite runs before focused filters. The active workflow records formatting drift, continues through executable tests for complete diagnostics, and enforces the formatting result before packaging. Superseded D5–D8 workflows remain manually runnable but cannot duplicate every push/PR; D9 audits every focused test filter against an actual Rust test. Source packaging also excludes and statically rejects Python bytecode caches, build outputs, staging trees, nested ZIPs, and temporary smoke files, including caches created by the workflow’s own `compileall` gate. See `D9-BUILD-FIX-REPORT.md`.

## Verification status

All non-Rust artifact checks and fresh-extraction static checks pass in the delivery environment. Rust/Cargo is unavailable there, so the corrected archive's compilation and test success are not represented as completed. The included GitHub Actions and Termux scripts are the authoritative executable verification path.

## Final CI hygiene correction

- Fixed a deterministic GitHub Actions failure where the live checkout's `.git`
  directory was incorrectly treated as release-archive contamination.
- Added separate repository and release-tree validators.
- Added an automated positive/negative hygiene regression harness.
