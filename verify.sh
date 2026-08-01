#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

echo "NIVRA D2 CUMULATIVE VERIFICATION"
echo "================================"
echo

if ! command -v python3 >/dev/null 2>&1; then
  echo "FAIL: python3 is required."
  echo "Termux fix: pkg install python -y"
  exit 1
fi

echo "[1/4] D1 regression"
python3 tools/spec_lint.py >/tmp/nivra-d1-lint.txt
cat /tmp/nivra-d1-lint.txt
echo "D1 regression: PASS"
echo

echo "[2/4] D2 architecture"
python3 tools/d2_spec_lint.py
echo

echo "[3/4] D2 report"
python3 tools/d2_report.py
echo

echo "[4/4] Delivery artifacts"
for file in README.md DELIVERY-REPORT.md ACCEPTANCE-CHECKLIST.md MANUAL-VERIFICATION.md LICENSE; do
  test -s "$file" || { echo "FAIL: missing $file"; exit 1; }
done
echo "Delivery artifacts: PASS"
echo
echo "D2 architecture integrity: PASS"
echo "D2 grammar integrity: PASS"
echo "D2 examples: PASS"
echo "★★★★★ D2 GOLDEN BUILD"
