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
excluded = {"target", ".git", "__pycache__", ".release-staging", "fresh-extract"}
with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as output:
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if any(part in excluded for part in relative.parts):
            continue
        if not path.is_file() or path.resolve() == archive:
            continue
        if path.suffix in {".zip", ".pyc"}:
            continue
        if path.name.startswith(".nivra-"):
            continue
        output.write(path, Path(root.name) / relative)
print(archive)
PY

unzip -q "$ARCHIVE" -d "$TEMP_ROOT"
EXTRACTED="$TEMP_ROOT/$(basename "$ROOT")"
cd "$EXTRACTED"
python3 tools/release_tree_lint.py
python3 tools/spec_lint.py
python3 tools/d2_spec_lint.py
python3 tools/d3_structure_lint.py
python3 tools/d4_structure_lint.py
python3 tools/d5_structure_lint.py
python3 tools/d6_structure_lint.py
python3 tools/d7_structure_lint.py
python3 tools/d8_structure_lint.py
python3 tools/d9_structure_lint.py
python3 tools/d9_hygiene_regression.py
python3 tools/d6_dependency_lint.py
for script in verify.sh scripts/*.sh; do
  bash -n "$script"
done
if command -v cargo >/dev/null 2>&1; then
  export RUSTFLAGS="${RUSTFLAGS:--D warnings}"
  cargo metadata --locked --format-version 1 --no-deps >/dev/null
  cargo fmt --all -- --check
  cargo check --workspace --all-targets --locked
  cargo test --workspace --all-targets --locked --no-fail-fast
  cargo build --workspace --release --locked
  NIVRA_BIN="$PWD/target/release/nivra" bash scripts/d9-smoke.sh
else
  echo "Rust execution skipped: cargo is unavailable."
fi
printf 'Fresh extraction static verification: PASS\n'
