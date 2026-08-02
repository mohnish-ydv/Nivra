#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
BIN="${NIVRA_BIN:-$ROOT/target/debug/nivra}"

if [[ ! -x "$BIN" ]]; then
  echo "FAIL: nivra binary not found: $BIN"
  exit 1
fi

printf 'NIVRA D8 CLI SMOKE\n'
printf '==================\n'

"$BIN" --version | grep -F 'nivra 0.8.0 (generics and traits D8)'
"$BIN" doctor | grep -F 'D8 status: OPERATIONAL'

for fixture in examples/d8/*.nva; do
  output="$($BIN check "$fixture" 2>&1)"
  printf '%s\n' "$output" | grep -F '0 errors' >/dev/null
  echo "valid: PASS $fixture"
done

codes=(GEN001 GEN002 GEN003 GEN004 GEN005 GEN006 TRT001 TRT002 TRT003 TRT004 TRT005 TRT006)
index=0
for fixture in examples/d8/invalid/*.nva; do
  expected="${codes[$index]}"
  set +e
  output="$($BIN check "$fixture" 2>&1)"
  status=$?
  set -e
  if [[ $status -ne 1 ]]; then
    echo "FAIL: $fixture exited $status, expected 1"
    printf '%s\n' "$output"
    exit 1
  fi
  printf '%s\n' "$output" | grep -F "error[$expected]" >/dev/null || {
    echo "FAIL: $fixture did not emit $expected"
    printf '%s\n' "$output"
    exit 1
  }
  echo "invalid: PASS $fixture -> $expected"
  index=$((index + 1))
done

for code in "${codes[@]}"; do
  "$BIN" explain "$code" | grep -F "$code:" >/dev/null
  echo "explain: PASS $code"
done

"$BIN" typecheck examples/d8/05_complete_generics_traits_tour.nva \
  --functions --types --nominals --traits > .nivra-d8-report.txt
for anchor in 'fn identity<T>' 'record Box<T>' 'trait Display' 'impl Display for User'; do
  grep -F "$anchor" .nivra-d8-report.txt >/dev/null
 done

"$BIN" typecheck examples/d8/05_complete_generics_traits_tour.nva --json \
  > .nivra-d8-graph.json
python3 -m json.tool .nivra-d8-graph.json >/dev/null
rm -f .nivra-d8-report.txt .nivra-d8-graph.json

printf 'D8 CLI smoke tests: PASS\n'
