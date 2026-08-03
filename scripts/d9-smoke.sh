#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
BIN="${NIVRA_BIN:-$ROOT/target/debug/nivra}"

if [[ ! -x "$BIN" ]]; then
  echo "FAIL: nivra binary not found: $BIN"
  exit 1
fi

printf 'NIVRA D9 OWNERSHIP CLI SMOKE\n'
printf '=============================\n'

"$BIN" --version | grep -F 'nivra 0.9.0 (ownership and borrow checking D9)'
"$BIN" doctor | grep -F 'D9 status: OPERATIONAL'

for fixture in examples/d9/*.nva; do
  output="$($BIN check "$fixture" 2>&1)"
  printf '%s\n' "$output" | grep -F '0 errors' >/dev/null
  echo "valid: PASS $fixture"
done

codes=(OWN001 OWN002 OWN006 OWN007 BOR001 BOR002 BOR003 BOR004 BOR005 BOR006 BOR007 BOR008 BOR009)
index=0
for fixture in examples/d9/invalid/*.nva; do
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

"$BIN" ownership examples/d9/05_complete_ownership_tour.nva \
  --bindings --events --drops > .nivra-d9-ownership-report.txt
for anchor in 'OWNERSHIP SUMMARY' 'OWNERSHIP BINDINGS' 'OWNERSHIP EVENTS' 'DEFER AND DROP PLAN'; do
  grep -F "$anchor" .nivra-d9-ownership-report.txt >/dev/null
done

"$BIN" ownership examples/d9/05_complete_ownership_tour.nva --json \
  > .nivra-d9-ownership-graph.json
python3 -m json.tool .nivra-d9-ownership-graph.json >/dev/null
for anchor in '"bindings"' '"events"' '"exit_actions"' '"moves"' '"borrows"'; do
  grep -F "$anchor" .nivra-d9-ownership-graph.json >/dev/null
done
rm -f .nivra-d9-ownership-report.txt .nivra-d9-ownership-graph.json

printf 'D9 ownership CLI smoke tests: PASS\n'
