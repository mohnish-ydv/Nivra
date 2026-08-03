# D9 Acceptance Checklist

## Completed artifact checks

- [x] D1–D8 source and regression assets remain cumulative.
- [x] Workspace and lockfile are synchronized at 0.9.0 across nine local crates.
- [x] No registry dependency or unsafe Rust block was introduced.
- [x] Parser and type checker support explicit `move`.
- [x] Copy/Move classification uses concrete generic substitution.
- [x] Whole moves, partial moves, reinitialization, and branch joins are modeled.
- [x] Shared/mutable borrow conflicts and owner-use/write conflicts are modeled.
- [x] Last-use borrow expiry and inner reference-scope expiry are modeled.
- [x] Deferred borrows remain live until scope exit.
- [x] Borrowed fields, borrowed enum payloads, ambiguous return origins, local escapes, and borrow-across-await are diagnosed.
- [x] Deterministic defer/drop plans separate drop necessity from move-only references.
- [x] OWN001, OWN002, OWN006, OWN007, and BOR001–BOR009 are implemented and explained.
- [x] Five valid and thirteen dedicated invalid D9 fixtures are included.
- [x] At least 167 cumulative Rust test declarations are present.
- [x] Focused D7, D8, D9 parser/type/ownership/CLI regressions are in the active workflow.
- [x] Complete tests use `--no-fail-fast`.
- [x] Release ZIP generation and fresh-extraction verification are in CI.
- [x] Static fresh-extraction verification passes in the delivery environment.

## Authoritative executable gate

- [ ] GitHub Actions `Verify D9 Ownership and Borrow Foundation` is green.
- [ ] Rust 1.74 formatting check passes.
- [ ] `cargo check --workspace --all-targets --locked` passes.
- [ ] `cargo test --workspace --all-targets --locked --no-fail-fast` passes.
- [ ] Release build and D9 CLI smoke suite pass.
- [ ] `bash scripts/termux-verify.sh` prints `★★★★★ D9 GOLDEN BUILD`.

## D10 gate

Do not begin D10 until the executable gate above is green and any real compiler/test failures are fixed with permanent regressions.
