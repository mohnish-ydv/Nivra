#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path


FORBIDDEN_DIRECTORIES = {
    ".git",
    ".release-staging",
    "__pycache__",
    "fresh-extract",
    "target",
}
FORBIDDEN_SUFFIXES = {".pyc", ".pyo", ".zip"}


def fail(message: str) -> None:
    print(f"FAIL: {message}")
    raise SystemExit(1)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Validate a freshly extracted Nivra source release tree."
    )
    parser.add_argument(
        "root",
        nargs="?",
        default=".",
        help="extracted release root (default: current directory)",
    )
    release_root = Path(parser.parse_args().root).resolve()
    if not release_root.is_dir():
        fail(f"release root is not a directory: {release_root}")

    for path in sorted(release_root.rglob("*")):
        relative = path.relative_to(release_root)
        if any(part in FORBIDDEN_DIRECTORIES for part in relative.parts):
            fail(f"forbidden release directory present: {relative}")
        if not path.is_file():
            continue
        if path.suffix in FORBIDDEN_SUFFIXES or path.name.startswith(".nivra-"):
            fail(f"forbidden generated release file present: {relative}")

    print("Release-tree hygiene: PASS")


if __name__ == "__main__":
    main()
