#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"
export CARGO_TERM_COLOR=never

printf 'NIVRA D9 CUMULATIVE VERIFICATION\n'
printf '================================\n\n'

if ! command -v python3 >/dev/null 2>&1; then
  echo "FAIL: python3 is required."
  echo "Termux: pkg install python -y"
  exit 1
fi

printf '[1/17] D1 specification regression\n'
python3 tools/spec_lint.py
printf 'D1 regression: PASS\n\n'

printf '[2/17] D2 architecture regression\n'
python3 tools/d2_spec_lint.py
printf 'D2 regression: PASS\n\n'

printf '[3/17] D3 compiler-foundation regression\n'
python3 tools/d3_structure_lint.py
printf 'D3 regression: PASS\n\n'

printf '[4/17] D4 parser and AST regression\n'
python3 tools/d4_structure_lint.py
printf 'D4 regression: PASS\n\n'

printf '[5/17] D5 semantic regression\n'
python3 tools/d5_structure_lint.py
printf 'D5 regression: PASS\n\n'

printf '[6/17] D6 type-checker regression\n'
python3 tools/d6_structure_lint.py
printf 'D6 regression: PASS\n\n'

printf '[7/17] D7 nominal and member regression\n'
python3 tools/d7_structure_lint.py
printf 'D7 regression: PASS\n\n'

printf '[8/17] D8 generics and trait regression\n'
python3 tools/d8_structure_lint.py
printf 'D8 regression: PASS\n\n'

printf '[9/17] D9 ownership and borrow structure\n'
python3 tools/d9_structure_lint.py
printf 'D9 structure: PASS\n\n'

printf '[10/17] Cargo dependency graph preflight\n'
python3 tools/d6_dependency_lint.py
printf 'Cargo dependency graph: PASS\n\n'

printf '[11/17] Script and metadata preflight\n'
python3 -m compileall -q tools
for script in verify.sh scripts/*.sh; do
  bash -n "$script"
done
printf 'Script and metadata preflight: PASS\n\n'

if ! command -v cargo >/dev/null 2>&1; then
  echo "FAIL: cargo is required for D9 executable verification."
  echo "Termux: pkg install rust -y"
  echo "Then run: bash scripts/termux-verify.sh"
  exit 1
fi

printf '[12/17] Cargo metadata and lockfile\n'
cargo metadata --locked --format-version 1 --no-deps >/dev/null
printf 'Cargo metadata: PASS\n\n'

printf '[13/17] Rust formatting\n'
if [[ "${NIVRA_SKIP_RUNNER_FORMAT:-0}" == "1" ]]; then
  cargo fmt --all -- --check
elif cargo fmt --version >/dev/null 2>&1; then
  cargo fmt --all
  cargo fmt --all -- --check
else
  echo 'Rust formatting: SKIPPED (rustfmt unavailable in this Termux package)'
fi
printf 'Rust formatting gate: PASS or explicitly unavailable\n\n'

printf '[14/17] Compile every workspace target\n'
cargo check --workspace --all-targets --locked
printf 'Rust compilation: PASS\n\n'

printf '[15/17] Complete Rust unit and integration tests\n'
cargo test --workspace --all-targets --locked --no-fail-fast
printf 'Rust tests: PASS\n\n'

printf '[16/17] Debug build and D9 CLI smoke tests\n'
cargo build --workspace --locked
bash scripts/d9-smoke.sh
printf '\n'

printf '[17/17] Delivery reports\n'
for report in d3 d4 d5 d6 d7 d8 d9; do
  python3 "tools/${report}_report.py"
  printf '\n'
done
printf 'D1 regression: PASS\n'
printf 'D2 regression: PASS\n'
printf 'D3 regression: PASS\n'
printf 'D4 regression: PASS\n'
printf 'D5 regression: PASS\n'
printf 'D6 regression: PASS\n'
printf 'D7 regression: PASS\n'
printf 'D8 regression: PASS\n'
printf 'D9 structure: PASS\n'
printf 'Rust compilation: PASS\n'
printf 'Rust tests: PASS\n'
printf 'D9 ownership CLI smoke tests: PASS\n'
printf '★★★★★ D9 GOLDEN BUILD\n'
