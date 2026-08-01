#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${NIVRA_D3_TEST_DIR:-$HOME/nivra-d3-verification}"

if ! command -v python3 >/dev/null 2>&1; then
  echo "FAIL: Python is missing."
  echo "Run: pkg install python -y"
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "FAIL: Rust/Cargo is missing."
  echo "Run: pkg install rust -y"
  exit 1
fi

if [[ "$ROOT" == "$DEST" ]]; then
  cd "$ROOT"
  export NIVRA_APPLY_FORMAT=1
  exec bash verify.sh
fi

case "$DEST" in
  "$HOME"/*) ;;
  *)
    echo "FAIL: NIVRA_D3_TEST_DIR must stay inside Termux home: $HOME"
    exit 1
    ;;
esac

echo "Copying D3 to Termux internal storage:"
echo "  $DEST"
rm -rf "$DEST"
mkdir -p "$DEST"
cp -R "$ROOT/." "$DEST/"
rm -rf "$DEST/target" "$DEST/.nivra-verify"

cd "$DEST"
export NIVRA_APPLY_FORMAT=1
exec bash verify.sh
