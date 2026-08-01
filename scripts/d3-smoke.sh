#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN="$ROOT/target/debug/nivra"
if [[ ! -x "$BIN" ]]; then
  echo "FAIL: $BIN is missing; run cargo build --workspace first"
  exit 1
fi

echo "CLI version"
"$BIN" --version | grep -F "nivra 0.3.0" >/dev/null

echo "Valid fixture checks"
for file in examples/d3/*.nva; do
  "$BIN" check "$file" >/dev/null
done

echo "Lossless lexer output"
"$BIN" lex examples/d3/02_unicode_and_comments.nva --trivia \
  | grep -F "doc_line_comment" >/dev/null
"$BIN" lex examples/d3/02_unicode_and_comments.nva --trivia \
  | grep -F "block_comment" >/dev/null

echo "Machine-readable output"
"$BIN" check examples/d3/03_literals_and_operators.nva --json \
  | python3 -c 'import json,sys; data=json.load(sys.stdin); assert data["errors"] == 0'

expect_failure() {
  local file="$1"
  local code="$2"
  local output
  local status

  set +e
  output="$("$BIN" check "$file" 2>&1)"
  status=$?
  set -e

  if [[ "$status" -ne 1 ]]; then
    echo "FAIL: expected exit code 1 for $file, got $status"
    echo "$output"
    exit 1
  fi
  if ! grep -F "$code" <<<"$output" >/dev/null; then
    echo "FAIL: expected $code for $file"
    echo "$output"
    exit 1
  fi
}

echo "Invalid fixture diagnostics"
expect_failure examples/d3/invalid/unterminated_string.nva LEX002
expect_failure examples/d3/invalid/malformed_numbers.nva LEX005
expect_failure examples/d3/invalid/unterminated_comment.nva LEX004

echo "Diagnostic explanations"
"$BIN" explain LEX005 | grep -F "Malformed number" >/dev/null

echo "CLI smoke tests: PASS"
