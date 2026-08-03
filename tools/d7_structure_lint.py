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
    path = ROOT / relative
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        fail(f"invalid JSON in {relative}: {exc}")

required = [
    "spec/d7/delivery.json",
    "spec/d7/nominal-model.json",
    "spec/d7/diagnostics.json",
    "spec/d7/checker-pipeline.json",
    "docs/43-D7-IMPLEMENTATION.md",
    "docs/44-RECORDS-STRUCTS-CONSTRUCTION.md",
    "docs/45-MEMBER-AND-METHOD-LOOKUP.md",
    "docs/46-ENUM-VARIANTS.md",
    "docs/47-NOMINAL-DIAGNOSTICS.md",
    "docs/48-D7-TO-D8-GATE.md",
    "scripts/d7-smoke.sh",
    "tools/d7_report.py",
    "D7-BUILD-FIX-REPORT.md",
    "D7-FINAL-FIX-REPORT.md",
    "D7-FORMAT-RELEASE-FIX-REPORT.md",
    ".github/workflows/verify-d7.yml",
]
missing = [item for item in required if not (ROOT / item).is_file()]
if missing:
    fail("missing D7 files: " + ", ".join(missing))
print("D7 required files: PASS")

delivery = load_json("spec/d7/delivery.json")
model = load_json("spec/d7/nominal-model.json")
diagnostics = load_json("spec/d7/diagnostics.json")
pipeline = load_json("spec/d7/checker-pipeline.json")
if delivery.get("delivery") != "D7" or delivery.get("version") != "0.7.0":
    fail("D7 delivery identity mismatch")
if set(model.get("nominal_kinds", [])) != {"record", "struct", "enum"}:
    fail("D7 nominal kinds mismatch")
if pipeline.get("record_expression_node") != "record_expression":
    fail("D7 record-expression pipeline metadata missing")
print("D7 metadata: PASS")

workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
if workspace["workspace"]["package"].get("version") != "0.9.0":
    fail("cumulative workspace version is not 0.9.0")
if workspace["workspace"]["package"].get("rust-version") != "1.74":
    fail("Rust version policy changed")
members = workspace["workspace"]["members"]
if len(members) != 9:
    fail(f"D7 should retain nine focused crates, found {len(members)}")
lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
if lock.count('version = "0.9.0"') != 9:
    fail("Cargo.lock does not contain nine 0.9.0 packages")
if "registry+" in lock or "checksum =" in lock:
    fail("D7 unexpectedly introduced registry dependencies")
print("D7 Cargo workspace: PASS")

syntax = (ROOT / "crates/nivra-syntax/src/lib.rs").read_text(encoding="utf-8")
parser = (ROOT / "crates/nivra-parser/src/lib.rs").read_text(encoding="utf-8")
types = (ROOT / "crates/nivra-types/src/lib.rs").read_text(encoding="utf-8")
cli = (ROOT / "crates/nivra-cli/src/main.rs").read_text(encoding="utf-8")
for anchor in [
    "RecordFieldInitializer",
    '"record_field_initializer"',
]:
    if anchor not in syntax:
        fail(f"D7 syntax anchor missing: {anchor}")
for anchor in [
    "fn looks_like_record_expression(",
    "fn parse_record_expression(",
    "SyntaxKind::RecordExpression",
    "SyntaxKind::RecordFieldInitializer",
]:
    if anchor not in parser:
        fail(f"D7 parser anchor missing: {anchor}")
for anchor in [
    "pub enum NominalKind",
    "pub struct FieldInfo",
    "pub struct VariantInfo",
    "pub struct MethodInfo",
    "pub struct NominalTypeInfo",
    "fn collect_nominals(",
    "fn attach_methods(",
    "fn infer_member(",
    "fn infer_record_expression(",
    "fn infer_enum_variant_call(",
    "fn is_mutable_place(",
    "pub fn nominal_report(",
]:
    if anchor not in types:
        fail(f"D7 type-checker anchor missing: {anchor}")
for anchor in [
    "--nominals",
    "NOMINAL TYPES AND MEMBERS",
    "Nominal types:",
    "D9 status: OPERATIONAL",
]:
    if anchor not in cli:
        fail(f"D7 CLI anchor missing: {anchor}")
print("D7 implementation anchors: PASS")

codes = [item["code"] for item in diagnostics.get("codes", [])]
expected = [f"NOM{number:03d}" for number in range(1, 11)]
if codes != expected or len(codes) != len(set(codes)):
    fail(f"D7 diagnostic inventory mismatch: {codes}")
for code in expected:
    if code not in types or code not in cli:
        fail(f"{code} is not implemented and explained")
print("D7 diagnostics: PASS")

valid = sorted((ROOT / "examples/d7").glob("*.nva"))
invalid = sorted((ROOT / "examples/d7/invalid").glob("*.nva"))
if len(valid) != 5 or len(invalid) != 10:
    fail(f"expected 5 valid and 10 invalid D7 fixtures, found {len(valid)} and {len(invalid)}")
for index, code in enumerate(expected, 1):
    fixture = invalid[index - 1]
    if not fixture.name.startswith(f"{index:02d}_"):
        fail(f"fixture ordering mismatch for {code}: {fixture.name}")
for path in valid + invalid:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("module "):
        fail(f"fixture lacks module declaration: {path.relative_to(ROOT)}")
print("D7 fixtures: PASS")

all_rust_paths = sorted((ROOT / "crates").rglob("*.rs"))
all_rust = "\n".join(path.read_text(encoding="utf-8") for path in all_rust_paths)
test_count = len(re.findall(r"#\[test\]", all_rust))
if test_count < 98:
    fail(f"expected at least 98 cumulative Rust tests, found {test_count}")
if re.search(r"(?m)^\s*unsafe\s*\{", all_rust):
    fail("unsafe Rust block found")
for forbidden in [".unwrap()", ".expect(", "std::sync::LazyLock", ".is_none_or("]:
    if forbidden in types:
        fail(f"forbidden or Rust-1.74-incompatible marker in nivra-types: {forbidden}")
print(f"D7 Rust test inventory: PASS ({test_count})")

# A lightweight delimiter scanner catches common archive corruption.
def check_rust(path: Path, text: str) -> None:
    pairs = {")": "(", "]": "[", "}": "{"}
    stack: list[tuple[str, int]] = []
    state = "code"
    depth = 0
    i = 0
    while i < len(text):
        c = text[i]
        n = text[i + 1] if i + 1 < len(text) else ""
        if state == "line":
            if c == "\n":
                state = "code"
            i += 1
            continue
        if state == "block":
            if c == "/" and n == "*":
                depth += 1
                i += 2
                continue
            if c == "*" and n == "/":
                depth -= 1
                i += 2
                if depth == 0:
                    state = "code"
                continue
            i += 1
            continue
        if state in {"string", "char"}:
            if c == "\\":
                i += 2
                continue
            if state == "string" and c == '"':
                following = text[i + 1] if i + 1 < len(text) else ""
                if following.isalnum() or following == "_":
                    line = text.count("\n", 0, i) + 1
                    fail(
                        f"invalid Rust string-literal suffix in "
                        f"{path.relative_to(ROOT)} at line {line}"
                    )
                state = "code"
            elif state == "char" and c == "'":
                state = "code"
            i += 1
            continue
        if c == "/" and n == "/":
            state = "line"
            i += 2
            continue
        if c == "/" and n == "*":
            state = "block"
            depth = 1
            i += 2
            continue
        if c == '"':
            state = "string"
            i += 1
            continue
        if c == "'":
            # Rust lifetimes are not character literals. Treat as a char only when
            # a nearby closing quote exists.
            closing = text.find("'", i + 1, min(len(text), i + 16))
            if closing != -1 and "\n" not in text[i + 1:closing]:
                state = "char"
            i += 1
            continue
        if c in "([{":
            stack.append((c, i))
        elif c in ")]}":
            if not stack or stack[-1][0] != pairs[c]:
                fail(f"unbalanced delimiter in {path.relative_to(ROOT)} at byte {i}")
            stack.pop()
        i += 1
    if state in {"string", "char", "block"} or stack:
        fail(f"unclosed Rust syntax state in {path.relative_to(ROOT)}")

for path in all_rust_paths:
    check_rust(path, path.read_text(encoding="utf-8"))
print("D7 Rust lexical preflight: PASS")

# Guard the two D6 failures and D7 parser ambiguity.
if "let ok = true" in types:
    fail("reserved keyword `ok` reintroduced as a binding")
if 'State.redy("done")' in types:
    fail("unescaped nested string regression reintroduced in enum-variant test")
if 'State.redy(\\\"done\\\")' not in types:
    fail("corrected enum-variant suggestion test fixture is missing")
types_manifest = tomllib.loads((ROOT / "crates/nivra-types/Cargo.toml").read_text(encoding="utf-8"))
parser_dev = types_manifest.get("dev-dependencies", {}).get("nivra-parser")
if not isinstance(parser_dev, dict) or parser_dev.get("path") != "../nivra-parser":
    fail("nivra-types parser test dependency regressed")
if "does_not_confuse_if_blocks_with_record_construction" not in parser:
    fail("record-expression/if-block ambiguity regression test missing")
if '.trim()\n            .rsplit("::")' not in parser:
    fail("empty record-construction leading-trivia fix is missing")
if "parses_empty_record_construction_after_leading_trivia" not in parser:
    fail("empty record-construction parser regression test missing")
if '"NOM001" => "The requested member (field, method, or enum variant) does not exist."' not in cli:
    fail("NOM001 public explanation does not include the member concept")
cli_tests = (ROOT / "crates/nivra-cli/tests/cli.rs").read_text(encoding="utf-8")
for regression_test in [
    "check_rejects_enum_record_construction_syntax",
    "explain_supports_nominal_diagnostics",
]:
    if regression_test not in cli_tests:
        fail(f"D7 CLI regression test missing: {regression_test}")
if "span: node.span()" not in types or "checks_method_bodies_against_their_signatures" not in types:
    fail("method signature-to-body mapping regression guard missing")
print("D6 build regressions and D7 ambiguity guards: PASS")

workflow = (ROOT / ".github/workflows/verify-d7.yml").read_text(encoding="utf-8")
for anchor in [
    "rustup toolchain install 1.74.0",
    "cargo metadata --locked --format-version 1 --no-deps",
    "python3 tools/d7_structure_lint.py",
    "cargo fmt --all -- --check",
    "cargo check --workspace --all-targets --locked",
    "cargo test -p nivra-parser parses_empty_record_construction_after_leading_trivia --locked",
    "cargo test -p nivra-types rejects_record_syntax_for_enum --locked",
    "cargo test -p nivra-cli --test cli explain_supports_nominal_diagnostics --locked",
    "cargo test -p nivra-cli --test cli check_rejects_enum_record_construction_syntax --locked",
    "cargo test --workspace --all-targets --locked --no-fail-fast",
    "bash verify.sh",
    "cargo build --workspace --release --locked",
    "bash scripts/d7-smoke.sh",
    "actions/upload-artifact@v4",
]:
    if anchor not in workflow:
        fail(f"D7 workflow missing {anchor!r}")
print("D7 CI contract: PASS")
print("D7 structure integrity: PASS")
