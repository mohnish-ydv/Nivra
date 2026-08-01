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

required_files = [
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "crates/nivra-source/Cargo.toml",
    "crates/nivra-source/src/lib.rs",
    "crates/nivra-diagnostics/Cargo.toml",
    "crates/nivra-diagnostics/src/lib.rs",
    "crates/nivra-lexer/Cargo.toml",
    "crates/nivra-lexer/src/lib.rs",
    "crates/nivra-cli/Cargo.toml",
    "crates/nivra-cli/src/main.rs",
    "crates/nivra-cli/tests/cli.rs",
    "spec/d3/delivery.json",
    "spec/d3/implementation.json",
    "spec/d3/diagnostics.json",
    "docs/19-D3-IMPLEMENTATION.md",
    "docs/20-SOURCE-MANAGER.md",
    "docs/21-DIAGNOSTICS-ENGINE.md",
    "docs/22-LEXER.md",
    "docs/23-CLI.md",
    "docs/24-D3-TO-D4-GATE.md",
    "scripts/d3-smoke.sh",
    "scripts/termux-verify.sh",
    "D3-QA-REPORT.md",
]
missing = [path for path in required_files if not (ROOT / path).is_file()]
if missing:
    fail("missing D3 files: " + ", ".join(missing))
print("D3 required files: PASS")

delivery = load_json("spec/d3/delivery.json")
implementation = load_json("spec/d3/implementation.json")
diagnostics = load_json("spec/d3/diagnostics.json")
if delivery.get("delivery") != "D3" or delivery.get("version") != "0.3.0":
    fail("D3 delivery identity mismatch")
if implementation.get("external_runtime_dependencies") != 0:
    fail("D3 must have zero external runtime dependencies")
if implementation.get("compiler_stages_implemented") != ["CP-01", "CP-02", "CP-03"]:
    fail("D3 implemented compiler stage list changed unexpectedly")
print("D3 metadata: PASS")

workspace = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
expected_crates = [
    "crates/nivra-source",
    "crates/nivra-diagnostics",
    "crates/nivra-lexer",
    "crates/nivra-cli",
]
for crate in expected_crates:
    if f'"{crate}"' not in workspace:
        fail(f"workspace missing {crate}")
if 'version = "0.5.0"' not in workspace:
    fail("cumulative workspace version is not 0.5.0")
if 'unsafe_code = "forbid"' not in workspace:
    fail("workspace must forbid unsafe Rust")
print("Rust workspace: PASS")

for manifest in [ROOT / "Cargo.toml", *sorted((ROOT / "crates").glob("*/Cargo.toml"))]:
    try:
        tomllib.loads(manifest.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as exc:
        fail(f"invalid TOML in {manifest.relative_to(ROOT)}: {exc}")
print("Cargo manifest syntax: PASS")

for manifest in sorted((ROOT / "crates").glob("*/Cargo.toml")):
    text = manifest.read_text(encoding="utf-8")
    dependencies_section = re.search(
        r"(?ms)^\[dependencies\]\s*(.*?)(?=^\[|\Z)", text
    )
    if dependencies_section:
        for line in dependencies_section.group(1).splitlines():
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            if "path =" not in stripped:
                fail(f"non-local dependency in {manifest.relative_to(ROOT)}: {stripped}")
print("Dependency isolation: PASS")

lock_text = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
if "registry+" in lock_text or "checksum =" in lock_text:
    fail("Cargo.lock unexpectedly contains a registry dependency")
for package in ["nivra-source", "nivra-diagnostics", "nivra-lexer", "nivra-cli"]:
    if f'name = "{package}"' not in lock_text:
        fail(f"Cargo.lock missing {package}")
print("Cargo lock integrity: PASS")

rust_files = sorted((ROOT / "crates").rglob("*.rs"))
combined_rust = "\n".join(path.read_text(encoding="utf-8") for path in rust_files)

def check_rust_delimiters(path: Path, text: str) -> None:
    pairs = {")": "(", "]": "[", "}": "{"}
    stack: list[tuple[str, int]] = []
    index = 0
    block_depth = 0
    state = "code"
    while index < len(text):
        char = text[index]
        next_char = text[index + 1] if index + 1 < len(text) else ""

        if state == "line_comment":
            if char == "\n":
                state = "code"
            index += 1
            continue
        if state == "block_comment":
            if char == "/" and next_char == "*":
                block_depth += 1
                index += 2
                continue
            if char == "*" and next_char == "/":
                block_depth -= 1
                index += 2
                if block_depth == 0:
                    state = "code"
                continue
            index += 1
            continue
        if state == "string":
            if char == "\\":
                index += 2
                continue
            if char == '"':
                state = "code"
            index += 1
            continue
        if state == "char":
            if char == "\\":
                index += 2
                continue
            if char == "'":
                state = "code"
            if char == "\n":
                fail(f"unterminated Rust character literal in {path.relative_to(ROOT)}")
            index += 1
            continue

        if char == "/" and next_char == "/":
            state = "line_comment"
            index += 2
            continue
        if char == "/" and next_char == "*":
            state = "block_comment"
            block_depth = 1
            index += 2
            continue
        if char == '"':
            state = "string"
            index += 1
            continue
        if char == "'":
            # Rust lifetimes such as `'a` have no closing quote. Treat a nearby
            # closing quote as a character literal; otherwise leave it as code.
            cursor = index + 1
            escaped = False
            closing = None
            while cursor < min(len(text), index + 16) and text[cursor] != "\n":
                if escaped:
                    escaped = False
                elif text[cursor] == "\\":
                    escaped = True
                elif text[cursor] == "'":
                    closing = cursor
                    break
                cursor += 1
            if closing is not None:
                state = "char"
            index += 1
            continue
        if char in "([{":
            stack.append((char, index))
        elif char in ")]}":
            if not stack or stack[-1][0] != pairs[char]:
                fail(f"unbalanced Rust delimiter `{char}` in {path.relative_to(ROOT)}")
            stack.pop()
        index += 1

    if state in {"string", "char", "block_comment"}:
        fail(f"unterminated Rust lexical construct in {path.relative_to(ROOT)}")
    if stack:
        fail(f"unclosed Rust delimiter in {path.relative_to(ROOT)}")

for rust_file in rust_files:
    rust_text = rust_file.read_text(encoding="utf-8")
    check_rust_delimiters(rust_file, rust_text)
    if "ELLIPSIZATION" in rust_text:
        fail(f"truncated source marker found in {rust_file.relative_to(ROOT)}")
print("Rust lexical preflight: PASS")
if re.search(r"(?m)^\s*unsafe\s*\{", combined_rust):
    fail("unsafe Rust block found in D3 compiler implementation")
if "SourceId" not in combined_rust or "SourceManager" not in combined_rust:
    fail("source manager API missing")
if "Renderer" not in combined_rust or "Diagnostic" not in combined_rust:
    fail("diagnostics API missing")
if "pub fn lex(" not in combined_rust or "TokenKind" not in combined_rust:
    fail("lexer API missing")
print("Safe Rust implementation anchors: PASS")

spec_keywords = {
    line.strip()
    for line in (ROOT / "spec/d2/keywords.txt").read_text(encoding="utf-8").splitlines()
    if line.strip()
}
lexer_text = (ROOT / "crates/nivra-lexer/src/lib.rs").read_text(encoding="utf-8")
implemented_keywords = set(
    re.findall(r'^\s*"([a-z_]+)"\s*=>\s*Self::', lexer_text, re.MULTILINE)
)
missing_keywords = sorted(spec_keywords - implemented_keywords)
extra_keywords = sorted(implemented_keywords - spec_keywords)
if missing_keywords or extra_keywords:
    fail(
        f"keyword parity mismatch; missing={missing_keywords}, extra={extra_keywords}"
    )
if len(spec_keywords) != 45:
    fail(f"expected 45 D2 keywords, found {len(spec_keywords)}")
print("D2/D3 keyword parity: PASS")

code_rows = diagnostics.get("codes", [])
codes = [row["code"] for row in code_rows]
if len(codes) != len(set(codes)) or len(codes) < 14:
    fail("D3 diagnostic codes are missing or duplicated")
for code in codes:
    if code not in combined_rust:
        fail(f"diagnostic code {code} is not implemented or explained")
print("Diagnostic code coverage: PASS")

valid_examples = sorted((ROOT / "examples/d3").glob("*.nva"))
invalid_examples = sorted((ROOT / "examples/d3/invalid").glob("*.nva"))
if len(valid_examples) != 4 or len(invalid_examples) != 3:
    fail(
        f"expected 4 valid and 3 invalid D3 fixtures, found "
        f"{len(valid_examples)} and {len(invalid_examples)}"
    )
for path in valid_examples + invalid_examples:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("module "):
        fail(f"fixture missing module declaration: {path.relative_to(ROOT)}")
print("D3 fixtures: PASS")

test_count = len(re.findall(r"#\[test\]", combined_rust))
if test_count < 15:
    fail(f"expected at least 15 Rust tests, found {test_count}")
print(f"Rust test inventory: PASS ({test_count})")

script_files = [ROOT / "verify.sh", *sorted((ROOT / "scripts").glob("*.sh"))]
for path in script_files:
    text = path.read_text(encoding="utf-8")
    if "/tmp/nivra-d1-lint.txt" in text:
        fail(f"fixed D2 temporary path remains in {path.relative_to(ROOT)}")
print("Termux permission regression: PASS")

workflow = ROOT / ".github/workflows/verify-d5.yml"
if not workflow.is_file():
    fail("cumulative GitHub Actions workflow missing")
workflow_text = workflow.read_text(encoding="utf-8")
for required in [
    "bash verify.sh",
    "cargo build --workspace --release",
    "actions/upload-artifact@v4",
]:
    if required not in workflow_text:
        fail(f"workflow missing {required!r}")
print("D3 CI regression contract: PASS")

print("D3 structure integrity: PASS")
