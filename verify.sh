#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"
export CARGO_TERM_COLOR=never

echo "NIVRA D3 CUMULATIVE VERIFICATION"
echo "================================"
echo

if ! command -v python3 >/dev/null 2>&1; then
  echo "FAIL: python3 is required."
  echo "Termux: pkg install python -y"
  exit 1
fi

echo "[1/7] D1 specification regression"
python3 tools/spec_lint.py
echo "D1 regression: PASS"
echo

echo "[2/7] D2 architecture regression"
python3 tools/d2_spec_lint.py
echo "D2 regression: PASS"
echo

echo "[3/7] D3 implementation structure"
python3 tools/d3_structure_lint.py
echo "D3 structure: PASS"
echo

if ! command -v cargo >/dev/null 2>&1; then
  echo "FAIL: cargo is required for D3."
  echo "Termux: pkg install rust -y"
  echo "Then run from internal storage with: bash scripts/termux-verify.sh"
  exit 1
fi

echo "[4/8] Script and source preflight"
python3 -m compileall -q tools
for script in verify.sh scripts/*.sh; do
  bash -n "$script"
done
echo "Script and source preflight: PASS"
echo

echo "[5/8] Rust formatting"
if cargo fmt --version >/dev/null 2>&1; then
  if [[ "${NIVRA_APPLY_FORMAT:-0}" == "1" ]]; then
    cargo fmt --all
  fi
  cargo fmt --all -- --check
  echo "Rust formatting: PASS"
elif [[ "${NIVRA_REQUIRE_FORMAT:-0}" == "1" ]]; then
  echo "FAIL: rustfmt is required in this verification environment"
  exit 1
else
  echo "Rust formatting: SKIPPED (rustfmt unavailable in this Termux package)"
fi
echo

echo "[6/8] Rust unit and integration tests"
cargo test --workspace --all-targets --locked
echo "Rust tests: PASS"
echo

echo "[7/8] Debug build and CLI smoke tests"
cargo build --workspace --locked
bash scripts/d3-smoke.sh
echo

echo "[8/8] Delivery report"
python3 tools/d3_report.py
echo
echo "D1 regression: PASS"
echo "D2 regression: PASS"
echo "D3 structure: PASS"
echo "Rust tests: PASS"
echo "CLI smoke tests: PASS"
echo "★★★★★ D3 GOLDEN BUILD"
