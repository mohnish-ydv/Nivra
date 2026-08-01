#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${NIVRA_BIN:-$ROOT/target/debug/nivra}"
WORK="$ROOT/.nivra-verify/d5-smoke"

if [[ ! -x "$BIN" ]]; then
  echo "FAIL: Nivra binary is missing: $BIN"
  exit 1
fi

rm -rf "$WORK"
mkdir -p "$WORK"

version="$($BIN --version)"
[[ "$version" == *"0.5.0"* && "$version" == *"D5"* ]] || {
  echo "FAIL: unexpected version output: $version"
  exit 1
}
echo "D5 version: PASS"

for suite in d2 d3 d4 d5; do
  for file in "$ROOT"/examples/$suite/*.nva; do
    output="$WORK/$suite-$(basename "$file").check.txt"
    "$BIN" check "$file" > "$output"
    grep -q "0 errors" "$output"
  done
done
echo "D2-D5 valid semantic regression fixtures: PASS"

"$BIN" resolve "$ROOT/examples/d5/05_complete_semantic_tour.nva" \
  --symbols --scopes > "$WORK/semantic-report.txt"
grep -q "Module: examples.d5.complete_semantic_tour" "$WORK/semantic-report.txt"
grep -q "SYMBOL TABLE" "$WORK/semantic-report.txt"
grep -q "SCOPE TREE" "$WORK/semantic-report.txt"
grep -q "function" "$WORK/semantic-report.txt"
grep -q "parameter" "$WORK/semantic-report.txt"
grep -q "local" "$WORK/semantic-report.txt"
echo "Semantic symbol and scope report: PASS"

"$BIN" resolve "$ROOT/examples/d5/01_module_index.nva" --json > "$WORK/semantic.json"
python3 - "$WORK/semantic.json" <<'PY'
import json
import sys
from pathlib import Path
payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
assert payload["module"] == "examples.d5.module_index"
assert isinstance(payload["symbols"], list)
assert isinstance(payload["scopes"], list)
assert isinstance(payload["resolutions"], list)
assert payload["errors"] == 0
assert payload["unresolved_names"] == 0
PY
echo "Semantic JSON: PASS"

"$BIN" check "$ROOT/examples/d5/01_module_index.nva" --json > "$WORK/check.json"
python3 - "$WORK/check.json" <<'PY'
import json
import sys
from pathlib import Path
payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
assert payload["errors"] == 0
assert payload["semantic_symbols"] > 0
assert payload["semantic_scopes"] >= 3
assert payload["resolved_names"] > 0
PY
echo "Semantic check JSON: PASS"

expect_failure() {
  local file="$1"
  local code="$2"
  local label="$3"
  set +e
  "$BIN" check "$file" > "$WORK/$label.stdout" 2> "$WORK/$label.stderr"
  local status=$?
  set -e
  if [[ $status -eq 0 ]]; then
    echo "FAIL: $label unexpectedly passed"
    exit 1
  fi
  grep -q "$code" "$WORK/$label.stderr" || {
    echo "FAIL: $label did not emit $code"
    cat "$WORK/$label.stderr"
    exit 1
  }
}

expect_failure "$ROOT/examples/d5/invalid/01_duplicate_module_symbol.nva" "SEM001" "duplicate-module"
expect_failure "$ROOT/examples/d5/invalid/02_duplicate_local.nva" "SEM002" "duplicate-local"
expect_failure "$ROOT/examples/d5/invalid/03_unresolved_name.nva" "SEM003" "unresolved-name"
expect_failure "$ROOT/examples/d5/invalid/04_use_before_declaration.nva" "SEM003" "use-before-declaration"
expect_failure "$ROOT/examples/d5/invalid/05_multiple_semantic_errors.nva" "SEM004" "multiple-module"
grep -q "SEM005" "$WORK/multiple-module.stderr"
grep -q "SEM006" "$WORK/multiple-module.stderr"
echo "Semantic diagnostics and recovery: PASS"

"$BIN" parse "$ROOT/examples/d4/04_lossless_comments.nva" > "$WORK/lossless.txt"
grep -q "Lossless round trip: PASS" "$WORK/lossless.txt"
echo "D4 lossless parser regression: PASS"

"$BIN" explain SEM003 > "$WORK/explain.txt"
grep -q "not visible" "$WORK/explain.txt"
echo "Semantic diagnostic explanation: PASS"

echo "D5 CLI smoke tests: PASS"
