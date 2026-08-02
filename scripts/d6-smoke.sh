#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${NIVRA_BIN:-$ROOT/target/debug/nivra}"
WORK="${NIVRA_SMOKE_DIR:-$ROOT/.nivra-d6-smoke}"

if [[ ! -x "$BIN" ]]; then
  echo "FAIL: Nivra binary is not executable: $BIN"
  exit 1
fi

rm -rf "$WORK"
mkdir -p "$WORK"
trap 'rm -rf "$WORK"' EXIT

printf 'D6 CLI SMOKE TESTS\n'
printf '==================\n'

"$BIN" --version | grep -F "0.7.0" >/dev/null
"$BIN" doctor | grep -F "D7 status: OPERATIONAL" >/dev/null
printf 'CLI identity: PASS\n'

for source in "$ROOT"/examples/d6/*.nva; do
  "$BIN" check "$source" >"$WORK/check.txt" 2>"$WORK/check.err"
  grep -F "0 errors" "$WORK/check.txt" >/dev/null
  "$BIN" typecheck "$source" >"$WORK/typecheck.txt" 2>"$WORK/typecheck.err"
  grep -F "Errors: 0" "$WORK/typecheck.txt" >/dev/null
done
printf 'Valid D6 fixtures: PASS\n'

check_invalid() {
  local file="$1"
  local code="$2"
  if "$BIN" typecheck "$file" >"$WORK/invalid.out" 2>"$WORK/invalid.err"; then
    echo "FAIL: invalid fixture unexpectedly passed: $file"
    exit 1
  fi
  grep -F "error[$code]" "$WORK/invalid.err" >/dev/null || {
    echo "FAIL: $file did not emit $code"
    cat "$WORK/invalid.err"
    exit 1
  }
}

check_invalid "$ROOT/examples/d6/invalid/01_binding_mismatch.nva" TYP001
check_invalid "$ROOT/examples/d6/invalid/02_bad_operator.nva" TYP002
check_invalid "$ROOT/examples/d6/invalid/03_wrong_arity.nva" TYP003
check_invalid "$ROOT/examples/d6/invalid/04_bad_argument.nva" TYP004
check_invalid "$ROOT/examples/d6/invalid/05_bad_return.nva" TYP005
check_invalid "$ROOT/examples/d6/invalid/06_cannot_infer_none.nva" TYP006
check_invalid "$ROOT/examples/d6/invalid/07_non_bool_condition.nva" TYP007
check_invalid "$ROOT/examples/d6/invalid/08_unknown_type.nva" TYP008
check_invalid "$ROOT/examples/d6/invalid/09_heterogeneous_array.nva" TYP009
check_invalid "$ROOT/examples/d6/invalid/10_immutable_assignment.nva" TYP010
printf 'TYP001-TYP010 conformance: PASS\n'

"$BIN" typecheck "$ROOT/examples/d6/05_complete_type_tour.nva" \
  --functions --types >"$WORK/report.txt"
grep -F "FUNCTION SIGNATURES" "$WORK/report.txt" >/dev/null
grep -F "INFERRED AND DECLARED BINDINGS" "$WORK/report.txt" >/dev/null
grep -F "fn clamp(value: Int, minimum: Int, maximum: Int) -> Int" "$WORK/report.txt" >/dev/null
printf 'Human type reports: PASS\n'

"$BIN" typecheck "$ROOT/examples/d6/02_functions_and_calls.nva" --json \
  >"$WORK/types.json"
python3 -m json.tool "$WORK/types.json" >/dev/null
grep -F '"return_type":"Int"' "$WORK/types.json" >/dev/null
printf 'JSON type graph: PASS\n'

"$BIN" explain TYP004 | grep -F "parameter type" >/dev/null
printf 'Diagnostic explanations: PASS\n'
printf 'D6 CLI smoke tests: PASS\n'
