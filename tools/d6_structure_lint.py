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
    "crates/nivra-types/Cargo.toml",
    "crates/nivra-types/src/lib.rs",
    "crates/nivra-cli/src/main.rs",
    "crates/nivra-cli/tests/cli.rs",
    "spec/d6/delivery.json",
    "spec/d6/type-model.json",
    "spec/d6/diagnostics.json",
    "spec/d6/checker-pipeline.json",
    "docs/37-D6-IMPLEMENTATION.md",
    "docs/38-TYPE-REPRESENTATION.md",
    "docs/39-LOCAL-INFERENCE.md",
    "docs/40-OPERATORS-CALLS-RETURNS.md",
    "docs/41-TYPE-DIAGNOSTICS.md",
    "docs/42-D6-TO-D7-GATE.md",
    "scripts/d6-smoke.sh",
    "scripts/termux-verify.sh",
    "tools/d6_dependency_lint.py",
    ".github/workflows/verify-d6.yml",
    "D6-QA-REPORT.md",
    "D6-BUILD-FIX-REPORT.md",
]
missing = [item for item in required if not (ROOT / item).is_file()]
if missing:
    fail("missing D6 files: " + ", ".join(missing))
print("D6 required files: PASS")


delivery = load_json("spec/d6/delivery.json")
model = load_json("spec/d6/type-model.json")
diagnostics = load_json("spec/d6/diagnostics.json")
pipeline = load_json("spec/d6/checker-pipeline.json")
if delivery.get("delivery") != "D6" or delivery.get("version") != "0.6.0":
    fail("D6 delivery identity mismatch")
if delivery.get("workspace_crates") != 8:
    fail("D6 workspace crate count must be eight")
if delivery.get("external_runtime_dependencies") != 0:
    fail("D6 must retain zero third-party runtime dependencies")
if delivery.get("type_checking_included") is not True:
    fail("D6 must include type checking")
if delivery.get("ownership_checking_included") is not False:
    fail("D6 must not claim ownership checking")
if len(model.get("primitive_families", [])) != 7:
    fail("D6 primitive type inventory is incomplete")
if len(pipeline.get("pipeline", [])) < 9:
    fail("D6 checker pipeline is incomplete")
print("D6 metadata: PASS")


workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
members = set(workspace["workspace"]["members"])
required_members = {
    "crates/nivra-source",
    "crates/nivra-diagnostics",
    "crates/nivra-lexer",
    "crates/nivra-syntax",
    "crates/nivra-parser",
    "crates/nivra-sema",
    "crates/nivra-types",
    "crates/nivra-ownership",
    "crates/nivra-cli",
}
if members != required_members:
    fail(f"D6 workspace members mismatch: {sorted(members ^ required_members)}")
package = workspace["workspace"]["package"]
if package.get("version") != "0.9.0":
    fail("workspace version is not 0.9.0")
if package.get("rust-version") != "1.74":
    fail("minimum Rust version changed unexpectedly")
if workspace["workspace"]["lints"]["rust"].get("unsafe_code") != "forbid":
    fail("workspace no longer forbids unsafe Rust")
toolchain = tomllib.loads((ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))
if toolchain.get("toolchain", {}).get("channel") != "1.74.0":
    fail("verification toolchain is not pinned to Rust 1.74.0")
print("D6 Rust workspace: PASS")


for manifest in [ROOT / "Cargo.toml", *sorted((ROOT / "crates").glob("*/Cargo.toml"))]:
    try:
        parsed = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as exc:
        fail(f"invalid TOML in {manifest.relative_to(ROOT)}: {exc}")
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        for dependency, value in parsed.get(section, {}).items():
            if not isinstance(value, dict) or "path" not in value:
                fail(
                    f"non-local {section} entry {dependency!r} "
                    f"in {manifest.relative_to(ROOT)}"
                )

types_manifest = tomllib.loads(
    (ROOT / "crates/nivra-types/Cargo.toml").read_text(encoding="utf-8")
)
types_dev_dependencies = types_manifest.get("dev-dependencies", {})
parser_dev_dependency = types_dev_dependencies.get("nivra-parser")
if not isinstance(parser_dev_dependency, dict) or parser_dev_dependency.get("path") != "../nivra-parser":
    fail("nivra-types tests require local nivra-parser dev-dependency")
print("D6 manifest and dependency isolation: PASS")


lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
if "registry+" in lock or "checksum =" in lock:
    fail("Cargo.lock contains registry content")
for package_name in [
    "nivra-source",
    "nivra-diagnostics",
    "nivra-lexer",
    "nivra-syntax",
    "nivra-parser",
    "nivra-sema",
    "nivra-types",
    "nivra-ownership",
    "nivra-cli",
]:
    if f'name = "{package_name}"' not in lock:
        fail(f"Cargo.lock missing {package_name}")
if lock.count('version = "0.9.0"') != 9:
    fail("Cargo.lock does not contain nine 0.9.0 packages")
types_lock_match = re.search(
    r'name = "nivra-types"\nversion = "0\.9\.0"\ndependencies = \[\n(?P<body>.*?)\n\]',
    lock,
    re.DOTALL,
)
if types_lock_match is None or '"nivra-parser"' not in types_lock_match.group("body"):
    fail("Cargo.lock is missing the nivra-types -> nivra-parser test edge")
print("D6 Cargo lock: PASS")


types_rs = (ROOT / "crates/nivra-types/src/lib.rs").read_text(encoding="utf-8")
cli_rs = (ROOT / "crates/nivra-cli/src/main.rs").read_text(encoding="utf-8")
combined = types_rs + "\n" + cli_rs
for anchor in [
    "pub enum Type",
    "pub struct FunctionSignature",
    "pub struct BindingType",
    "pub struct TypeCheckResult",
    "pub fn check(",
    "fn collect_signatures(",
    "fn infer_expression(",
    "fn infer_binary(",
    "fn infer_call(",
    "fn require_assignable(",
    '"typecheck" => typecheck_command',
    "fn typecheck_json(",
]:
    if anchor not in combined:
        fail(f"D6 implementation anchor missing: {anchor}")
print("D6 implementation anchors: PASS")


codes = [item["code"] for item in diagnostics.get("codes", [])]
expected_codes = [f"TYP{number:03d}" for number in range(1, 11)]
if codes != expected_codes or len(codes) != len(set(codes)):
    fail(f"D6 diagnostic inventory mismatch: {codes}")
for code in codes:
    if code not in types_rs or code not in cli_rs:
        fail(f"D6 diagnostic {code} is not implemented and explained")
print("D6 type diagnostics: PASS")


valid = sorted((ROOT / "examples/d6").glob("*.nva"))
invalid = sorted((ROOT / "examples/d6/invalid").glob("*.nva"))
if len(valid) != 5 or len(invalid) != 10:
    fail(f"expected 5 valid and 10 invalid D6 fixtures, found {len(valid)} and {len(invalid)}")
for path in valid + invalid:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("module "):
        fail(f"fixture missing module declaration: {path.relative_to(ROOT)}")
for index, code in enumerate(expected_codes, 1):
    fixture = invalid[index - 1]
    if f"{index:02d}_" not in fixture.name:
        fail(f"D6 fixture ordering mismatch for {code}: {fixture.name}")
print("D6 fixtures: PASS")


all_rust_paths = sorted((ROOT / "crates").rglob("*.rs"))
all_rust = "\n".join(path.read_text(encoding="utf-8") for path in all_rust_paths)
test_count = len(re.findall(r"#\[test\]", all_rust))
if test_count < 75:
    fail(f"expected at least 75 cumulative Rust tests, found {test_count}")
if re.search(r"(?m)^\s*unsafe\s*\{", all_rust):
    fail("unsafe Rust block found in compiler implementation")
for forbidden in [".unwrap()", ".expect("]:
    if forbidden in types_rs:
        fail(f"forbidden panic convenience {forbidden!r} in nivra-types")
for incompatible in [".is_none_or(", "let_chains", "std::sync::LazyLock"]:
    if incompatible in types_rs:
        fail(f"Rust 1.74-incompatible API marker found: {incompatible}")
print(f"Cumulative Rust test inventory: PASS ({test_count})")


# Lightweight delimiter scanner catches common packaging corruption without Rust.
def check_rust_delimiters(path: Path, text: str) -> None:
    pairs = {")": "(", "]": "[", "}": "{"}
    stack: list[tuple[str, int]] = []
    state = "code"
    block_depth = 0
    index = 0
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
                fail(f"unbalanced Rust delimiter in {path.relative_to(ROOT)} at byte {index}")
            stack.pop()
        index += 1
    if state in {"string", "char", "block_comment"} or stack:
        fail(f"unclosed Rust syntax state in {path.relative_to(ROOT)}")


for rust_path in all_rust_paths:
    check_rust_delimiters(rust_path, rust_path.read_text(encoding="utf-8"))
print("D6 Rust lexical preflight: PASS")


for rust_path in all_rust_paths:
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
                f"suspicious duplicate Rust line in {rust_path.relative_to(ROOT)}:"
                f"{line_number}: {stripped!r}"
            )
        previous = stripped
print("D6 Rust copy-integrity check: PASS")


for forbidden in [
    "/tmp/nivra-d1-lint.txt",
    "TO" + "DO",
    "T" + "BD",
    "FIX" + "ME",
    "ELLIP" + "SIZATION",
]:
    for path in [
        ROOT / "crates/nivra-types/src/lib.rs",
        ROOT / "crates/nivra-cli/src/main.rs",
        ROOT / "verify.sh",
        ROOT / "scripts/termux-verify.sh",
        ROOT / "scripts/d6-smoke.sh",
    ]:
        if forbidden in path.read_text(encoding="utf-8"):
            fail(f"forbidden marker {forbidden!r} in {path.relative_to(ROOT)}")
print("D6 release hygiene: PASS")



type_source = (ROOT / "crates/nivra-types/src/lib.rs").read_text(encoding="utf-8")
if "let ok = true" in type_source:
    fail("D6 primitive inference test uses reserved keyword `ok` as a binding name")
for required in [
    "let enabled = true",
    'binding.name == "enabled" && binding.ty == Type::Bool',
]:
    if required not in type_source:
        fail(f"D6 primitive Bool inference regression guard missing {required!r}")
print("D6 reserved-keyword fixture regression: PASS")


workflow = (ROOT / ".github/workflows/verify-d6.yml").read_text(encoding="utf-8")
for anchor in [
    "bash verify.sh",
    "rustup toolchain install 1.74.0",
    "python3 tools/d6_dependency_lint.py",
    "cargo metadata --locked --format-version 1 --no-deps",
    "cargo check --workspace --all-targets --locked",
    "cargo test --workspace --all-targets --locked --no-fail-fast",
    "cargo build --workspace --release --locked",
    "bash scripts/d6-smoke.sh",
    "actions/upload-artifact@v4",
]:
    if anchor not in workflow:
        fail(f"D6 workflow missing {anchor!r}")
print("D6 CI contract: PASS")

print("D6 structure integrity: PASS")
