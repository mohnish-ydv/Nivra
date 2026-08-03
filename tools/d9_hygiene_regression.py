#!/usr/bin/env python3
from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXCLUDED_DIRS = {
    ".git",
    ".release-staging",
    "__pycache__",
    "fresh-extract",
    "target",
}


def fail(message: str, output: str = "") -> None:
    print(f"FAIL: {message}")
    if output:
        print(output.rstrip())
    raise SystemExit(1)


def ignore_generated(_directory: str, names: list[str]) -> set[str]:
    ignored = {name for name in names if name in EXCLUDED_DIRS}
    ignored.update(
        name
        for name in names
        if name.endswith((".pyc", ".pyo", ".zip")) or name.startswith(".nivra-")
    )
    return ignored


def run(script: Path, cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(script)],
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="nivra-d9-hygiene-") as temporary:
        temp = Path(temporary)
        checkout = temp / "checkout"
        shutil.copytree(ROOT, checkout, ignore=ignore_generated)
        (checkout / ".git").mkdir()
        (checkout / ".git" / "HEAD").write_text("ref: refs/heads/main\n", encoding="utf-8")
        (checkout / "target").mkdir()
        (checkout / "target" / "placeholder").write_text("generated\n", encoding="utf-8")
        (checkout / "tools" / "__pycache__").mkdir()
        (checkout / "tools" / "__pycache__" / "placeholder.pyc").write_bytes(b"cache")

        checkout_result = run(checkout / "tools" / "d9_structure_lint.py", checkout)
        if checkout_result.returncode != 0:
            fail(
                "D9 structure lint rejected a legitimate live Git checkout",
                checkout_result.stdout,
            )

        release = temp / "release"
        shutil.copytree(ROOT, release, ignore=ignore_generated)
        release_result = run(release / "tools" / "release_tree_lint.py", release)
        if release_result.returncode != 0:
            fail("clean source release failed hygiene validation", release_result.stdout)

        (release / ".git").mkdir()
        contaminated_result = run(release / "tools" / "release_tree_lint.py", release)
        if contaminated_result.returncode == 0:
            fail("release hygiene accepted a packaged .git directory")
        if "forbidden release directory present: .git" not in contaminated_result.stdout:
            fail(
                "release hygiene rejected contamination with the wrong diagnostic",
                contaminated_result.stdout,
            )

    print("D9 repository/release hygiene regression: PASS")


if __name__ == "__main__":
    main()
