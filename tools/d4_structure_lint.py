#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import sys
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


required_files = [
    "crates/nivra-syntax/Cargo.toml",
    "crates/nivra-syntax/src/lib.rs",
    "crates/nivra-parser/Cargo.toml",
    "crates/nivra-parser/src/lib.rs",
    "crates/nivra-cli/src/main.rs",
    "crates/nivra-cli/tests/cli.rs",
    "spec/d4/delivery.json",
    "spec/d4/parser.json",
    "spec/d4/diagnostics.json",
    "spec/d4/syntax-tree.json",
    "docs/25-D4-IMPLEMENTATION.md",
    "docs/26-LOSSLESS-CST.md",
    "docs/27-PARSER-ARCHITECTURE.md",
    "docs/28-AST-FOUNDATION.md",
    "docs/29-ERROR-RECOVERY.md",
    "docs/30-D4-TO-D5-GATE.md",
    "scripts/d4-smoke.sh",
    "scripts/termux-verify.sh",
    ".github/workflows/verify-d5.yml",
]
missing = [path for path in required_files if not (ROOT / path).is_file()]
if missing:
    fail("missing D4 files: " + ", ".join(missing))
print("D4 required files: PASS")

delivery = load_json("spec/d4/delivery.json")
parser_spec = load_json("spec/d4/parser.json")
diagnostics = load_json("spec/d4/diagnostics.json")
tree_spec = load_json("spec/d4/syntax-tree.json")
if delivery.get("delivery") != "D4" or delivery.get("version") != "0.4.0":
    fail("D4 delivery identity mismatch")
if parser_spec.get("external_runtime_dependencies") != 0:
    fail("D4 parser must keep zero external runtime dependencies")
if not parser_spec.get("lossless") or not parser_spec.get("preserves_trivia"):
    fail("D4 parser must remain lossless and preserve trivia")
print("D4 metadata: PASS")

workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
members = set(workspace.get("workspace", {}).get("members", []))
required_members = {
    "crates/nivra-source",
    "crates/nivra-diagnostics",
    "crates/nivra-lexer",
    "crates/nivra-syntax",
    "crates/nivra-parser",
    "crates/nivra-cli",
}
if not required_members.issubset(members):
    fail(f"workspace missing crates: {sorted(required_members - members)}")
package = workspace.get("workspace", {}).get("package", {})
if package.get("version") != "0.5.0":
    fail("workspace version is not 0.5.0")
if workspace.get("workspace", {}).get("lints", {}).get("rust", {}).get("unsafe_code") != "forbid":
    fail("workspace no longer forbids unsafe Rust")
print("D4 Rust workspace: PASS")

for manifest in [ROOT / "Cargo.toml", *sorted((ROOT / "crates").glob("*/Cargo.toml"))]:
    try:
        parsed = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as exc:
        fail(f"invalid TOML in {manifest.relative_to(ROOT)}: {exc}")
    for dependency, value in parsed.get("dependencies", {}).items():
        if not isinstance(value, dict) or "path" not in value:
            fail(f"non-local dependency {dependency!r} in {manifest.relative_to(ROOT)}")
print("D4 manifest and dependency isolation: PASS")

lock_text = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
if "registry+" in lock_text or "checksum =" in lock_text:
    fail("Cargo.lock contains a registry dependency")
for package_name in [
    "nivra-source",
    "nivra-diagnostics",
    "nivra-lexer",
    "nivra-syntax",
    "nivra-parser",
    "nivra-cli",
]:
    if f'name = "{package_name}"' not in lock_text:
        fail(f"Cargo.lock missing {package_name}")
print("D4 Cargo lock: PASS")

syntax_text = (ROOT / "crates/nivra-syntax/src/lib.rs").read_text(encoding="utf-8")
parser_text = (ROOT / "crates/nivra-parser/src/lib.rs").read_text(encoding="utf-8")
cli_text = (ROOT / "crates/nivra-cli/src/main.rs").read_text(encoding="utf-8")
combined = syntax_text + "\n" + parser_text + "\n" + cli_text

for anchor in [
    "pub enum SyntaxKind",
    "pub enum SyntaxElement",
    "pub struct SyntaxNode",
    "pub trait AstNode",
    "pub struct ParseResult",
    "pub fn parse(",
    "fn parse_expression(",
    "fn infix_binding_power(",
    "fn recover_until(",
    '"parse" => parse_command',
]:
    if anchor not in combined:
        fail(f"D4 implementation anchor missing: {anchor}")
print("Parser and syntax implementation anchors: PASS")

required_kinds = set(tree_spec.get("required_node_kinds", []))
implemented_kinds = set(re.findall(r'=>\s*"([a-z_]+)",', syntax_text))
missing_kinds = sorted(required_kinds - implemented_kinds)
if missing_kinds:
    fail(f"syntax kinds missing from implementation: {missing_kinds}")
if len(required_kinds) < 55:
    fail(f"expected at least 55 required syntax kinds, found {len(required_kinds)}")
print(f"Syntax kind coverage: PASS ({len(required_kinds)})")

codes = [row["code"] for row in diagnostics.get("codes", [])]
if len(codes) != len(set(codes)) or len(codes) != 5:
    fail("D4 parser diagnostic codes are missing or duplicated")
for code in codes:
    if code not in combined:
        fail(f"parser diagnostic {code} is not implemented or explained")
print("Parser diagnostic coverage: PASS")

valid = sorted((ROOT / "examples/d4").glob("*.nva"))
invalid = sorted((ROOT / "examples/d4/invalid").glob("*.nva"))
if len(valid) != 5 or len(invalid) != 4:
    fail(f"expected 5 valid and 4 invalid D4 fixtures, found {len(valid)} and {len(invalid)}")
for path in valid + invalid:
    if not path.read_text(encoding="utf-8").startswith("module "):
        fail(f"D4 fixture missing module declaration: {path.relative_to(ROOT)}")
print("D4 fixtures: PASS")

all_rust = "\n".join(path.read_text(encoding="utf-8") for path in sorted((ROOT / "crates").rglob("*.rs")))
test_count = len(re.findall(r"#\[test\]", all_rust))
if test_count < 30:
    fail(f"expected at least 30 cumulative Rust tests, found {test_count}")
if re.search(r"(?m)^\s*unsafe\s*\{", all_rust):
    fail("unsafe Rust block found in compiler implementation")
print(f"Cumulative Rust test inventory: PASS ({test_count})")

for forbidden in ["/tmp/nivra-d1-lint.txt", "TO" + "DO", "T" + "BD", "FIX" + "ME", "ELLIP" + "SIZATION"]:
    for path in [
        ROOT / "crates/nivra-syntax/src/lib.rs",
        ROOT / "crates/nivra-parser/src/lib.rs",
        ROOT / "crates/nivra-cli/src/main.rs",
        ROOT / "verify.sh",
        ROOT / "scripts/termux-verify.sh",
    ]:
        if forbidden in path.read_text(encoding="utf-8"):
            fail(f"forbidden marker {forbidden!r} in {path.relative_to(ROOT)}")
print("D4 release hygiene: PASS")

workflow = (ROOT / ".github/workflows/verify-d5.yml").read_text(encoding="utf-8")
for anchor in [
    "bash verify.sh",
    "cargo build --workspace --release --locked",
    "actions/upload-artifact@v4",
]:
    if anchor not in workflow:
        fail(f"D4 workflow missing {anchor!r}")
print("D4 CI contract: PASS")

print("D4 structure integrity: PASS")
