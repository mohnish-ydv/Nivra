# D9 Build-Fix Report — Repository/Release Hygiene Separation

## Round-two CI failure

The uploaded GitHub Actions log failed before Cargo compilation because
`tools/d9_structure_lint.py` scanned the live repository checkout and rejected
GitHub's required root `.git` directory as though it had leaked into a release
archive. That made every real Actions checkout fail deterministically.

## Corrective action

- Removed release-archive hygiene scanning from the live source-structure lint.
- Added `tools/release_tree_lint.py` for freshly extracted source releases only.
- Wired the dedicated release lint after ZIP extraction in GitHub Actions and
  `scripts/fresh-extract-verify.sh`.
- Added structural guards that reject any future attempt to run release hygiene
  against the live checkout.
- Added a synthetic Git-checkout regression during final QA.

---

# Nivra D9 Build-Fix Report

## Supplied CI failure

The uploaded GitHub Actions run reached the D9 ownership regressions after successfully installing Rust 1.74, validating the cumulative source structure, validating Cargo metadata, formatting/checking the workspace, compiling every target, and passing the focused D7, D8, parser, type, and first two ownership regressions.

The run then failed in `rejects_shared_then_mutable_borrow_conflict`. The intended borrow checker diagnostic was never reached because the embedded Nivra program used call-style record construction:

```nva
Note(text: "a")
```

D7 established brace construction:

```nva
Note { text: "a" }
```

The type checker therefore correctly stopped earlier with `TYP006` for the dependent bindings.

## Corrections

1. Replaced every accidental call-style D9 record construction in ownership unit tests and D9 examples with the established brace syntax.
2. Added a D9 structure regression that discovers locally declared records/structs and rejects named-field call syntax before CI reaches Rust execution.
3. Removed the unused ownership `SyntaxElement` import reported by the supplied run.
4. Removed the unused `type_contains_generic` helper reported by the supplied run.
5. Made Rust warnings fatal in GitHub Actions, the golden verifier, fresh-extraction verification, and Termux verification.
6. Moved the complete workspace test suite with `--no-fail-fast` before individually filtered regressions, so one failed focused command cannot conceal later failures.
7. Changed formatting gates to verification-only; CI no longer mutates an unformatted checkout and then reports success.
8. Updated checkout and artifact actions from the Node-20-based majors reported as deprecated in the supplied log to checkout v6 and upload-artifact v7.
9. Updated all cumulative structure-lint workflow anchors to match the action migration.
10. Extended fresh-extraction verification to run formatting, warning-free compilation, the complete test suite, release build, and D9 CLI smoke tests whenever Cargo is available.
11. Removed a false-golden Termux path: missing `rustfmt` is now a hard failure instead of a skipped check followed by `GOLDEN BUILD`.
12. Made the D9 workflow record a formatting failure, continue through compile/tests for broader diagnostics, and enforce the recorded failure before release packaging.
13. Hardened fresh-extract packaging against caches, staging directories, nested ZIPs, bytecode, and temporary `.nivra-*` files.
14. Enforced D5–D8 as manual-only superseded workflows, leaving D9 as the single authoritative push/PR pipeline.
15. Added a workflow filter audit that proves every focused `cargo test` command names a real Rust test and remains lockfile-pinned.
16. Fixed a release-hygiene defect where the workflow ran `compileall` before packaging and could copy `tools/__pycache__/*.pyc` into the source ZIP. The packager and D9 structure gate now reject caches, bytecode, nested archives, staging trees, build outputs, and temporary `.nivra-*` files.

## Verification boundary

All static, metadata, fixture, workflow, shell, Python, and fresh-archive checks pass in the artifact environment. The corrected Rust workspace could not be executed here because this sandbox has no Rust toolchain and network installation is unavailable. Therefore the corrected release still requires one authoritative GitHub Actions or Termux run before D9 can be called a golden executable build.
