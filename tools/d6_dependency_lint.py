#!/usr/bin/env python3
"""Validate the local Cargo graph without requiring Cargo or rustc.

This catches missing path dependencies used by Rust source and stale local edges in
Cargo.lock. It exists because the original D6 archive imported nivra_parser from
nivra-types tests without declaring nivra-parser as a dev-dependency.
"""
from __future__ import annotations

import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATES = ROOT / "crates"


def fail(message: str) -> None:
    print(f"FAIL: {message}")
    raise SystemExit(1)


def crate_ident(package_name: str) -> str:
    return package_name.replace("-", "_")


def local_dependencies(manifest: dict) -> dict[str, str]:
    output: dict[str, str] = {}
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        for name, value in manifest.get(section, {}).items():
            if not isinstance(value, dict) or "path" not in value:
                fail(f"{name!r} in {section} is not an isolated path dependency")
            output[name] = str(value["path"])
    return output


workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
members = workspace.get("workspace", {}).get("members", [])
if not isinstance(members, list) or not members:
    fail("workspace member list is missing")

package_to_manifest: dict[str, Path] = {}
package_dependencies: dict[str, set[str]] = {}
for member in members:
    manifest_path = ROOT / member / "Cargo.toml"
    if not manifest_path.is_file():
        fail(f"workspace manifest missing: {member}/Cargo.toml")
    manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
    package_name = manifest.get("package", {}).get("name")
    if not isinstance(package_name, str):
        fail(f"package name missing in {manifest_path.relative_to(ROOT)}")
    if package_name in package_to_manifest:
        fail(f"duplicate package name {package_name!r}")
    package_to_manifest[package_name] = manifest_path
    dependencies = local_dependencies(manifest)
    package_dependencies[package_name] = set(dependencies)
    for dependency, relative_path in dependencies.items():
        dependency_manifest = (manifest_path.parent / relative_path / "Cargo.toml").resolve()
        if not dependency_manifest.is_file():
            fail(
                f"{package_name} dependency {dependency!r} points to missing "
                f"{dependency_manifest}"
            )
        target = tomllib.loads(dependency_manifest.read_text(encoding="utf-8"))
        actual_name = target.get("package", {}).get("name")
        if actual_name != dependency:
            fail(
                f"{package_name} dependency key {dependency!r} targets package "
                f"{actual_name!r}"
            )

# Every local crate imported by source must be declared by the importing package.
known_by_ident = {crate_ident(name): name for name in package_to_manifest}
for package_name, manifest_path in package_to_manifest.items():
    source_root = manifest_path.parent / "src"
    test_root = manifest_path.parent / "tests"
    rust_paths = list(source_root.rglob("*.rs")) if source_root.exists() else []
    if test_root.exists():
        rust_paths.extend(test_root.rglob("*.rs"))
    imports: set[str] = set()
    for rust_path in rust_paths:
        text = rust_path.read_text(encoding="utf-8")
        imports.update(re.findall(r"\b(?:use|extern\s+crate)\s+(nivra_[A-Za-z0-9_]+)", text))
    declared = package_dependencies[package_name]
    for imported_ident in sorted(imports):
        imported_package = known_by_ident.get(imported_ident)
        if imported_package is None or imported_package == package_name:
            continue
        if imported_package not in declared:
            fail(
                f"{package_name} imports {imported_ident} but does not declare "
                f"{imported_package} in dependencies or dev-dependencies"
            )

# Parse local package dependency lists from Cargo.lock.
lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
lock_blocks = re.split(r"(?m)^\[\[package\]\]\s*$", lock)[1:]
locked_dependencies: dict[str, set[str]] = {}
for block in lock_blocks:
    name_match = re.search(r'(?m)^name = "([^"]+)"$', block)
    if name_match is None:
        continue
    body_match = re.search(r"(?ms)^dependencies = \[\n(?P<body>.*?)^\]$", block)
    dependencies: set[str] = set()
    if body_match is not None:
        dependencies.update(re.findall(r'^ "([^"]+)",?$', body_match.group("body"), re.MULTILINE))
    locked_dependencies[name_match.group(1)] = dependencies

for package_name, expected in sorted(package_dependencies.items()):
    actual = locked_dependencies.get(package_name)
    if actual is None:
        fail(f"Cargo.lock is missing package {package_name}")
    # All dependencies in this zero-external-dependency workspace are local, so exact
    # equality is correct and catches both missing and stale edges.
    if actual != expected:
        missing = sorted(expected - actual)
        extra = sorted(actual - expected)
        fail(
            f"Cargo.lock dependency mismatch for {package_name}: "
            f"missing={missing}, extra={extra}"
        )

print(f"D6 Cargo dependency graph: PASS ({len(package_to_manifest)} crates)")
