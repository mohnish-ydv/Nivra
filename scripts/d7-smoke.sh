#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${NIVRA_BIN:-$ROOT/target/debug/nivra}"
WORK="${NIVRA_SMOKE_DIR:-$ROOT/.nivra-d7-smoke}"

if [[ ! -x "$BIN" ]]; then
  echo "FAIL: Nivra binary is not executable: $BIN"
  exit 1
fi

rm -rf "$WORK"
mkdir -p "$WORK"
trap 'rm -rf "$WORK"' EXIT

printf 'D7 NOMINAL CLI SMOKE TESTS\n'
printf '==========================\n'

"$BIN" --version | grep -F "0.7.0" >/dev/null
"$BIN" doctor | grep -F "D7 status: OPERATIONAL" >/dev/null
printf 'CLI identity: PASS\n'

# The previous delivery remains executable.
NIVRA_BIN="$BIN" NIVRA_SMOKE_DIR="$WORK/d6" bash "$ROOT/scripts/d6-smoke.sh"
printf 'D6 CLI regression: PASS\n'

for source in "$ROOT"/examples/d7/*.nva; do
  "$BIN" check "$source" >"$WORK/check.txt" 2>"$WORK/check.err"
  grep -F "0 errors" "$WORK/check.txt" >/dev/null
  "$BIN" typecheck "$source" >"$WORK/typecheck.txt" 2>"$WORK/typecheck.err"
  grep -F "Errors: 0" "$WORK/typecheck.txt" >/dev/null
done
printf 'Valid D7 fixtures: PASS\n'

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

check_invalid "$ROOT/examples/d7/invalid/01_unknown_member.nva" NOM001
check_invalid "$ROOT/examples/d7/invalid/02_optional_member_access.nva" NOM002
check_invalid "$ROOT/examples/d7/invalid/03_missing_required_field.nva" NOM003
check_invalid "$ROOT/examples/d7/invalid/04_unknown_constructor_field.nva" NOM004
check_invalid "$ROOT/examples/d7/invalid/05_duplicate_constructor_field.nva" NOM005
check_invalid "$ROOT/examples/d7/invalid/06_constructor_field_type.nva" NOM006
check_invalid "$ROOT/examples/d7/invalid/07_enum_variant_payload.nva" NOM007
check_invalid "$ROOT/examples/d7/invalid/08_immutable_member_mutation.nva" NOM008
check_invalid "$ROOT/examples/d7/invalid/09_unknown_constructor.nva" NOM009
check_invalid "$ROOT/examples/d7/invalid/10_enum_record_syntax.nva" NOM010
printf 'NOM001-NOM010 conformance: PASS\n'

"$BIN" typecheck "$ROOT/examples/d7/05_complete_nominal_tour.nva" \
  --functions --types --nominals >"$WORK/report.txt"
grep -F "NOMINAL TYPES AND MEMBERS" "$WORK/report.txt" >/dev/null
grep -F "record Profile" "$WORK/report.txt" >/dev/null
grep -F "method add_score" "$WORK/report.txt" >/dev/null
grep -F "variant online(Profile)" "$WORK/report.txt" >/dev/null
printf 'Human nominal report: PASS\n'

"$BIN" typecheck "$ROOT/examples/d7/05_complete_nominal_tour.nva" --json \
  >"$WORK/nominals.json"
python3 -m json.tool "$WORK/nominals.json" >/dev/null
grep -F '"nominals":[' "$WORK/nominals.json" >/dev/null
grep -F '"mutable_receiver":true' "$WORK/nominals.json" >/dev/null
printf 'JSON nominal graph: PASS\n'

"$BIN" parse "$ROOT/examples/d7/01_records_and_construction.nva" --tree \
  >"$WORK/tree.txt"
grep -F "record_expression" "$WORK/tree.txt" >/dev/null
grep -F "record_field_initializer" "$WORK/tree.txt" >/dev/null
printf 'Record-expression CST: PASS\n'

"$BIN" explain NOM001 | grep -F "member" >/dev/null
"$BIN" explain NOM007 | grep -F "variant" >/dev/null
printf 'Nominal diagnostic explanations: PASS\n'
printf 'D7 CLI smoke tests: PASS\n'
