#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${NIVRA_BIN:-$ROOT/target/debug/nivra}"
WORK="$ROOT/.nivra-verify/d4-smoke"

if [[ ! -x "$BIN" ]]; then
  echo "FAIL: Nivra binary is missing: $BIN"
  exit 1
fi

rm -rf "$WORK"
mkdir -p "$WORK"

version="$($BIN --version)"
[[ "$version" == *"0.4.0"* && "$version" == *"D4"* ]] || {
  echo "FAIL: unexpected version output: $version"
  exit 1
}
echo "D4 version: PASS"

for suite in d2 d3 d4; do
  for file in "$ROOT"/examples/$suite/*.nva; do
    output="$WORK/$suite-$(basename "$file").check.txt"
    "$BIN" check "$file" > "$output"
    grep -q "0 errors" "$output"
  done
done
echo "D2-D4 valid parser regression fixtures: PASS"

"$BIN" parse "$ROOT/examples/d4/04_lossless_comments.nva" > "$WORK/lossless.txt"
grep -q "Lossless round trip: PASS" "$WORK/lossless.txt"
echo "Lossless CST round trip: PASS"

"$BIN" parse "$ROOT/examples/d4/02_expression_precedence.nva" --tree > "$WORK/tree.txt"
grep -q "function_declaration" "$WORK/tree.txt"
grep -q "binary_expression" "$WORK/tree.txt"
grep -q "if_expression" "$WORK/tree.txt"
echo "CST shape: PASS"

"$BIN" parse "$ROOT/examples/d4/04_lossless_comments.nva" --tree --trivia > "$WORK/trivia-tree.txt"
grep -q "doc_line_comment" "$WORK/trivia-tree.txt"
grep -q "block_comment" "$WORK/trivia-tree.txt"
echo "Trivia preservation: PASS"

"$BIN" parse "$ROOT/examples/d4/01_declarations.nva" --json > "$WORK/tree.json"
python3 - "$WORK/tree.json" <<'PY'
import json
import sys
from pathlib import Path
payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
assert payload["tree"]["kind"] == "source_file"
assert payload["errors"] == 0
assert isinstance(payload["tree"]["children"], list)
PY
echo "Parser JSON: PASS"

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

expect_failure "$ROOT/examples/d4/invalid/01_missing_block_close.nva" "PAR003" "missing-block"
expect_failure "$ROOT/examples/d4/invalid/02_missing_expression.nva" "PAR005" "missing-expression"
expect_failure "$ROOT/examples/d4/invalid/03_broken_declaration.nva" "PAR004" "broken-declaration"
echo "Parser diagnostics: PASS"

set +e
"$BIN" parse "$ROOT/examples/d4/invalid/03_broken_declaration.nva" --tree \
  > "$WORK/recovery-tree.txt" 2> "$WORK/recovery.stderr"
recovery_status=$?
set -e
if [[ $recovery_status -eq 0 ]]; then
  echo "FAIL: recovery fixture unexpectedly passed"
  exit 1
fi
grep -q "error" "$WORK/recovery-tree.txt"
grep -q "function_declaration" "$WORK/recovery-tree.txt"
echo "Multi-declaration recovery: PASS"

echo "D4 CLI smoke tests: PASS"
