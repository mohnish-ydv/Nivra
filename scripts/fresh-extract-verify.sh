#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARCHIVE="${1:-$ROOT/Nivra-D9-fresh-extract.zip}"
TEMP_ROOT="$(mktemp -d)"
trap 'rm -rf "$TEMP_ROOT"' EXIT

cd "$ROOT"
rm -f "$ARCHIVE"
python3 - "$ROOT" "$ARCHIVE" <<'PY'
from pathlib import Path
import sys
import zipfile
root = Path(sys.argv[1]).resolve()
archive = Path(sys.argv[2]).resolve()
excluded = {"target", ".git", "__pycache__"}
with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as output:
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if any(part in excluded for part in relative.parts):
            continue
        if path.is_file() and path.resolve() != archive:
            output.write(path, Path(root.name) / relative)
print(archive)
PY

unzip -q "$ARCHIVE" -d "$TEMP_ROOT"
EXTRACTED="$TEMP_ROOT/$(basename "$ROOT")"
cd "$EXTRACTED"
python3 tools/d9_structure_lint.py
python3 tools/d6_dependency_lint.py
for script in verify.sh scripts/*.sh; do
  bash -n "$script"
done
if command -v cargo >/dev/null 2>&1; then
  cargo metadata --locked --format-version 1 --no-deps >/dev/null
  cargo check --workspace --all-targets --locked
  cargo test --workspace --all-targets --locked --no-fail-fast
else
  echo "Rust execution skipped: cargo is unavailable."
fi
printf 'Fresh extraction static verification: PASS\n'
