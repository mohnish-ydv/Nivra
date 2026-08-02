#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"
export CARGO_TERM_COLOR=never

printf 'NIVRA D7 CUMULATIVE VERIFICATION\n'
printf '================================\n\n'

if ! command -v python3 >/dev/null 2>&1; then
  echo "FAIL: python3 is required."
  echo "Termux: pkg install python -y"
  exit 1
fi

printf '[1/13] D1 specification regression\n'
python3 tools/spec_lint.py
printf 'D1 regression: PASS\n\n'

printf '[2/13] D2 architecture regression\n'
python3 tools/d2_spec_lint.py
printf 'D2 regression: PASS\n\n'

printf '[3/13] D3 compiler-foundation regression\n'
python3 tools/d3_structure_lint.py
printf 'D3 regression: PASS\n\n'

printf '[4/13] D4 parser and AST regression\n'
python3 tools/d4_structure_lint.py
printf 'D4 regression: PASS\n\n'

printf '[5/13] D5 semantic regression\n'
python3 tools/d5_structure_lint.py
printf 'D5 regression: PASS\n\n'

printf '[6/13] D6 type-checker regression\n'
python3 tools/d6_structure_lint.py
printf 'D6 regression: PASS\n\n'

printf '[7/13] D7 nominal and member structure\n'
python3 tools/d7_structure_lint.py
printf 'D7 structure: PASS\n\n'

printf '[8/13] Cargo dependency graph preflight\n'
python3 tools/d6_dependency_lint.py
printf 'Cargo dependency graph: PASS\n\n'

if ! command -v cargo >/dev/null 2>&1; then
  echo "FAIL: cargo is required for D7."
  echo "Termux: pkg install rust -y"
  echo "Then run: bash scripts/termux-verify.sh"
  exit 1
fi

printf '[9/13] Script and source preflight\n'
python3 -m compileall -q tools
for script in verify.sh scripts/*.sh; do
  bash -n "$script"
done
printf 'Script and source preflight: PASS\n\n'

printf '[10/13] Rust formatting\n'
if cargo fmt --version >/dev/null 2>&1; then
  cargo fmt --all -- --check
  printf 'Rust formatting: PASS\n'
elif [[ "${NIVRA_REQUIRE_FORMAT:-0}" == "1" ]]; then
  echo "FAIL: rustfmt is required in this verification environment"
  exit 1
else
  echo "Rust formatting: SKIPPED (rustfmt unavailable in this Termux package)"
fi
printf '\n'

printf '[11/13] Rust unit and integration tests\n'
cargo test --workspace --all-targets --locked --no-fail-fast
printf 'Rust tests: PASS\n\n'

printf '[12/13] Debug build and D7 CLI smoke tests\n'
cargo build --workspace --locked
bash scripts/d7-smoke.sh
printf '\n'

printf '[13/13] Delivery reports\n'
python3 tools/d3_report.py
printf '\n'
python3 tools/d4_report.py
printf '\n'
python3 tools/d5_report.py
printf '\n'
python3 tools/d6_report.py
printf '\n'
python3 tools/d7_report.py
printf '\n'
printf 'D1 regression: PASS\n'
printf 'D2 regression: PASS\n'
printf 'D3 regression: PASS\n'
printf 'D4 regression: PASS\n'
printf 'D5 regression: PASS\n'
printf 'D6 regression: PASS\n'
printf 'D7 structure: PASS\n'
printf 'Rust tests: PASS\n'
printf 'D7 CLI smoke tests: PASS\n'
printf '★★★★★ D7 GOLDEN BUILD\n'
