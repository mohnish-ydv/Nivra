#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def fail(message: str) -> None:
    print(f"FAIL: {message}")
    raise SystemExit(1)

def load_json(relative: str):
    try:
        return json.loads((ROOT / relative).read_text(encoding="utf-8"))
    except Exception as exc:
        fail(f"invalid JSON in {relative}: {exc}")

required = [
    "crates/nivra-sema/Cargo.toml",
    "crates/nivra-sema/src/lib.rs",
    "crates/nivra-cli/src/main.rs",
    "crates/nivra-cli/tests/cli.rs",
    "spec/d5/delivery.json",
    "spec/d5/semantic-model.json",
    "spec/d5/diagnostics.json",
    "docs/31-D5-IMPLEMENTATION.md",
    "docs/32-SEMANTIC-AST.md",
    "docs/33-SCOPES-AND-SYMBOLS.md",
    "docs/34-NAME-RESOLUTION.md",
    "docs/35-SEMANTIC-DIAGNOSTICS.md",
    "docs/36-D5-TO-D6-GATE.md",
    "scripts/d5-smoke.sh",
    "scripts/termux-verify.sh",
    ".github/workflows/verify-d5.yml",
    "D5-QA-REPORT.md",
]
missing = [item for item in required if not (ROOT / item).is_file()]
if missing:
    fail("missing D5 files: " + ", ".join(missing))
print("D5 required files: PASS")

delivery = load_json("spec/d5/delivery.json")
model = load_json("spec/d5/semantic-model.json")
diagnostics = load_json("spec/d5/diagnostics.json")
if delivery.get("delivery") != "D5" or delivery.get("version") != "0.5.0":
    fail("D5 delivery identity mismatch")
if delivery.get("external_runtime_dependencies") != 0:
    fail("D5 must retain zero external runtime dependencies")
if delivery.get("type_checking_included") is not False:
    fail("D5 must not claim type checking")
if len(model.get("scope_kinds", [])) < 9 or len(model.get("symbol_kinds", [])) < 18:
    fail("semantic model inventory is incomplete")
print("D5 metadata: PASS")

workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
members = set(workspace["workspace"]["members"])
required_members = {
    "crates/nivra-source",
    "crates/nivra-diagnostics",
    "crates/nivra-lexer",
    "crates/nivra-syntax",
    "crates/nivra-parser",
    "crates/nivra-sema",
    "crates/nivra-cli",
}
if not required_members.issubset(members):
    fail(f"workspace missing D5 crates: {sorted(required_members - members)}")
if workspace["workspace"]["package"].get("version") != "0.9.0":
    fail("workspace version is not 0.9.0")
if workspace["workspace"]["package"].get("rust-version") != "1.74":
    fail("workspace minimum Rust version changed unexpectedly")
toolchain = tomllib.loads((ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))
if toolchain.get("toolchain", {}).get("channel") != "1.74.0":
    fail("D5 verification toolchain is not pinned to Rust 1.74.0")
if workspace["workspace"]["lints"]["rust"].get("unsafe_code") != "forbid":
    fail("workspace no longer forbids unsafe Rust")
print("D5 Rust workspace: PASS")

for manifest in [ROOT / "Cargo.toml", *sorted((ROOT / "crates").glob("*/Cargo.toml"))]:
    try:
        parsed = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as exc:
        fail(f"invalid TOML in {manifest.relative_to(ROOT)}: {exc}")
    for dependency, value in parsed.get("dependencies", {}).items():
        if not isinstance(value, dict) or "path" not in value:
            fail(f"non-local dependency {dependency!r} in {manifest.relative_to(ROOT)}")
print("D5 manifest and dependency isolation: PASS")

lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
if "registry+" in lock or "checksum =" in lock:
    fail("Cargo.lock contains registry content")
for package in [
    "nivra-source", "nivra-diagnostics", "nivra-lexer", "nivra-syntax",
    "nivra-parser", "nivra-sema", "nivra-cli",
]:
    if f'name = "{package}"' not in lock:
        fail(f"Cargo.lock missing {package}")
if lock.count('version = "0.9.0"') < 7:
    fail("Cargo.lock does not retain the D5 package set at 0.9.0")
print("D5 Cargo lock: PASS")

syntax = (ROOT / "crates/nivra-syntax/src/lib.rs").read_text(encoding="utf-8")
sema = (ROOT / "crates/nivra-sema/src/lib.rs").read_text(encoding="utf-8")
cli = (ROOT / "crates/nivra-cli/src/main.rs").read_text(encoding="utf-8")
combined = syntax + "\n" + sema + "\n" + cli
for anchor in [
    "pub trait NamedAstNode",
    "pub struct SymbolId",
    "pub struct ScopeId",
    "pub enum Namespace",
    "pub struct SemanticResult",
    "pub fn analyze(",
    "fn index_module(",
    "fn resolve_name_expression(",
    "fn closest_visible_name(",
    '"resolve" => resolve_command',
    "fn semantic_json(",
]:
    if anchor not in combined:
        fail(f"D5 implementation anchor missing: {anchor}")
print("D5 semantic implementation anchors: PASS")

# Catch accidental copy/paste corruption that delimiter-only checks cannot see.
for rust_path in sorted((ROOT / "crates").rglob("*.rs")):
    previous = None
    for line_number, line in enumerate(rust_path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.strip()
        if (
            stripped
            and stripped == previous
            and not stripped.endswith((";", ",", "{", "}"))
            and stripped not in {")", "]", "})"}
            and not stripped.startswith("//")
        ):
            fail(
                f"suspicious consecutive duplicate Rust line in "
                f"{rust_path.relative_to(ROOT)}:{line_number}: {stripped!r}"
            )
        previous = stripped
print("D5 Rust copy-integrity check: PASS")

codes = [item["code"] for item in diagnostics.get("codes", [])]
if len(codes) != 6 or len(codes) != len(set(codes)):
    fail("D5 diagnostic code inventory is invalid")
for code in codes:
    if code not in sema or code not in cli:
        fail(f"semantic code {code} is not implemented and explained")
print("D5 semantic diagnostics: PASS")

valid = sorted((ROOT / "examples/d5").glob("*.nva"))
invalid = sorted((ROOT / "examples/d5/invalid").glob("*.nva"))
if len(valid) != 5 or len(invalid) != 5:
    fail(f"expected 5 valid and 5 invalid D5 fixtures, found {len(valid)} and {len(invalid)}")
for path in valid + invalid:
    if not path.read_text(encoding="utf-8").startswith("module "):
        fail(f"fixture missing module declaration: {path.relative_to(ROOT)}")
print("D5 fixtures: PASS")

all_rust = "\n".join(path.read_text(encoding="utf-8") for path in sorted((ROOT / "crates").rglob("*.rs")))
test_count = len(re.findall(r"#\[test\]", all_rust))
if test_count < 50:
    fail(f"expected at least 50 cumulative Rust tests, found {test_count}")
if re.search(r"(?m)^\s*unsafe\s*\{", all_rust):
    fail("unsafe Rust block found in compiler implementation")
for forbidden in [".unwrap()", ".expect("]:
    if forbidden in sema:
        fail(f"forbidden panic convenience {forbidden!r} in semantic crate")
print(f"Cumulative Rust test inventory: PASS ({test_count})")

for forbidden in ["/tmp/nivra-d1-lint.txt", "TO" + "DO", "T" + "BD", "FIX" + "ME", "ELLIP" + "SIZATION"]:
    for path in [
        ROOT / "crates/nivra-sema/src/lib.rs",
        ROOT / "crates/nivra-cli/src/main.rs",
        ROOT / "verify.sh",
        ROOT / "scripts/termux-verify.sh",
    ]:
        if forbidden in path.read_text(encoding="utf-8"):
            fail(f"forbidden marker {forbidden!r} in {path.relative_to(ROOT)}")
print("D5 release hygiene: PASS")

workflow = (ROOT / ".github/workflows/verify-d5.yml").read_text(encoding="utf-8")
for anchor in [
    "bash verify.sh",
    "rustup toolchain install 1.74.0",
    "cargo build --workspace --release --locked",
    "bash scripts/d5-smoke.sh",
    "actions/upload-artifact@v7",
]:
    if anchor not in workflow:
        fail(f"D5 workflow missing {anchor!r}")
print("D5 CI contract: PASS")

print("D5 structure integrity: PASS")
