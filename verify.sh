#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"
export CARGO_TERM_COLOR=never

printf 'NIVRA D5 CUMULATIVE VERIFICATION\n'
printf '================================\n\n'

if ! command -v python3 >/dev/null 2>&1; then
  echo "FAIL: python3 is required."
  echo "Termux: pkg install python -y"
  exit 1
fi

printf '[1/10] D1 specification regression\n'
python3 tools/spec_lint.py
printf 'D1 regression: PASS\n\n'

printf '[2/10] D2 architecture regression\n'
python3 tools/d2_spec_lint.py
printf 'D2 regression: PASS\n\n'

printf '[3/10] D3 compiler-foundation regression\n'
python3 tools/d3_structure_lint.py
printf 'D3 regression: PASS\n\n'

printf '[4/10] D4 parser and AST regression\n'
python3 tools/d4_structure_lint.py
printf 'D4 regression: PASS\n\n'

printf '[5/10] D5 semantic structure\n'
python3 tools/d5_structure_lint.py
printf 'D5 structure: PASS\n\n'

if ! command -v cargo >/dev/null 2>&1; then
  echo "FAIL: cargo is required for D5."
  echo "Termux: pkg install rust -y"
  echo "Then run: bash scripts/termux-verify.sh"
  exit 1
fi

printf '[6/10] Script and source preflight\n'
python3 -m compileall -q tools
for script in verify.sh scripts/*.sh; do
  bash -n "$script"
done
printf 'Script and source preflight: PASS\n\n'

printf '[7/10] Rust formatting\n'
if cargo fmt --version >/dev/null 2>&1; then
  if [[ "${NIVRA_APPLY_FORMAT:-0}" == "1" ]]; then
    cargo fmt --all
  fi
  cargo fmt --all -- --check
  printf 'Rust formatting: PASS\n'
elif [[ "${NIVRA_REQUIRE_FORMAT:-0}" == "1" ]]; then
  echo "FAIL: rustfmt is required in this verification environment"
  exit 1
else
  echo "Rust formatting: SKIPPED (rustfmt unavailable in this Termux package)"
fi
printf '\n'

printf '[8/10] Rust unit and integration tests\n'
cargo test --workspace --all-targets --locked
printf 'Rust tests: PASS\n\n'

printf '[9/10] Debug build and D5 CLI smoke tests\n'
cargo build --workspace --locked
bash scripts/d5-smoke.sh
printf '\n'

printf '[10/10] Delivery reports\n'
python3 tools/d3_report.py
printf '\n'
python3 tools/d4_report.py
printf '\n'
python3 tools/d5_report.py
printf '\n'
printf 'D1 regression: PASS\n'
printf 'D2 regression: PASS\n'
printf 'D3 regression: PASS\n'
printf 'D4 regression: PASS\n'
printf 'D5 structure: PASS\n'
printf 'Rust tests: PASS\n'
printf 'D5 CLI smoke tests: PASS\n'
printf '★★★★★ D5 GOLDEN BUILD\n'
