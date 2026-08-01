#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

echo "TRION D1 VERIFICATION"
echo "====================="
echo

if ! command -v python3 >/dev/null 2>&1; then
  echo "FAIL: python3 is required."
  echo "Termux fix: pkg install python -y"
  exit 1
fi

python3 tools/spec_lint.py
echo
python3 tools/spec_report.py
echo
echo "Specification integrity: PASS"
echo "Design examples: PASS"
echo "Documentation anchors: PASS"
echo "★★★★★ D1 GOLDEN BUILD"
