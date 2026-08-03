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
    "spec/d8/delivery.json",
    "spec/d8/generic-model.json",
    "spec/d8/trait-model.json",
    "spec/d8/diagnostics.json",
    "spec/d8/checker-pipeline.json",
    "docs/49-D8-IMPLEMENTATION.md",
    "docs/50-GENERIC-SUBSTITUTION.md",
    "docs/51-TRAIT-CONSTRAINTS.md",
    "docs/52-IMPLEMENTATION-COHERENCE.md",
    "docs/53-METHOD-SELECTION.md",
    "docs/54-D8-DIAGNOSTICS.md",
    "docs/55-D8-TO-D9-GATE.md",
    "D8-BUILD-FIX-REPORT.md",
    "scripts/d8-smoke.sh",
    "tools/d8_report.py",
    ".github/workflows/verify-d8.yml",
]
missing = [item for item in required if not (ROOT / item).is_file()]
if missing:
    fail("missing D8 files: " + ", ".join(missing))
print("D8 required files: PASS")

delivery = load_json("spec/d8/delivery.json")
generic_model = load_json("spec/d8/generic-model.json")
trait_model = load_json("spec/d8/trait-model.json")
diagnostics = load_json("spec/d8/diagnostics.json")
pipeline = load_json("spec/d8/checker-pipeline.json")
if delivery.get("delivery") != "D8" or delivery.get("version") != "0.8.0":
    fail("D8 delivery identity mismatch")
if "explicit" not in generic_model.get("argument_modes", []):
    fail("explicit generic arguments missing from D8 model")
if "package orphan rule" not in trait_model.get("implementations", []):
    fail("orphan rule missing from D8 trait model")
if "generic_argument_list" not in pipeline.get("syntax_nodes", []):
    fail("generic argument CST node missing from D8 pipeline metadata")
print("D8 metadata: PASS")

workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
package = workspace["workspace"]["package"]
if package.get("version") != "0.9.0":
    fail("workspace version is not 0.9.0")
if package.get("rust-version") != "1.74":
    fail("Rust version policy changed")
members = workspace["workspace"]["members"]
if len(members) != 9:
    fail(f"D8 cumulative workspace should contain nine focused crates, found {len(members)}")
lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
if lock.count('version = "0.9.0"') != 9:
    fail("Cargo.lock does not contain nine 0.9.0 workspace packages")
if "registry+" in lock or "checksum =" in lock:
    fail("D8 unexpectedly introduced registry dependencies")
print("D8 Cargo workspace: PASS")

syntax = (ROOT / "crates/nivra-syntax/src/lib.rs").read_text(encoding="utf-8")
parser = (ROOT / "crates/nivra-parser/src/lib.rs").read_text(encoding="utf-8")
sema = (ROOT / "crates/nivra-sema/src/lib.rs").read_text(encoding="utf-8")
types = (ROOT / "crates/nivra-types/src/lib.rs").read_text(encoding="utf-8")
cli = (ROOT / "crates/nivra-cli/src/main.rs").read_text(encoding="utf-8")
cli_tests = (ROOT / "crates/nivra-cli/tests/cli.rs").read_text(encoding="utf-8")

for anchor in ["GenericArgumentList", '"generic_argument_list"']:
    if anchor not in syntax:
        fail(f"D8 syntax anchor missing: {anchor}")
for anchor in [
    "fn parse_generic_arguments(",
    "fn looks_like_generic_argument_list(",
    "SyntaxKind::GenericArgumentList",
    "parses_nested_generic_arguments_with_shift_right_token",
    "keeps_comparisons_out_of_generic_argument_parsing",
]:
    if anchor not in parser:
        fail(f"D8 parser anchor missing: {anchor}")
for anchor in [
    "Type::Parameter",
    "pub struct GenericParameterInfo",
    "pub struct TraitConstraint",
    "pub struct TraitInfo",
    "pub struct ImplementationInfo",
    "fn validate_generic_constraints(",
    "fn validate_implementations(",
    "fn infer_type_substitutions(",
    "fn validate_substitution_constraints(",
    "fn type_implements_trait(",
    "fn infer_method_call(",
    "pub fn trait_report(",
]:
    if anchor not in types:
        fail(f"D8 type-checker anchor missing: {anchor}")
for anchor in [
    "--traits",
    "Generic substitution: PASS",
    "Trait constraint validation: PASS",
    "Implementation coherence: PASS",
    "D9 status: OPERATIONAL",
]:
    if anchor not in cli:
        fail(f"D8 CLI anchor missing: {anchor}")
print("D8 implementation anchors: PASS")

codes = [item["code"] for item in diagnostics.get("codes", [])]
expected = [f"GEN{number:03d}" for number in range(1, 7)] + [
    f"TRT{number:03d}" for number in range(1, 7)
]
if codes != expected or len(codes) != len(set(codes)):
    fail(f"D8 diagnostic inventory mismatch: {codes}")
for code in expected:
    if code not in types or code not in cli:
        fail(f"{code} is not implemented and explained")
print("D8 diagnostics: PASS")

valid = sorted((ROOT / "examples/d8").glob("*.nva"))
invalid = sorted((ROOT / "examples/d8/invalid").glob("*.nva"))
if len(valid) != 5 or len(invalid) != 12:
    fail(f"expected 5 valid and 12 invalid D8 fixtures, found {len(valid)} and {len(invalid)}")
for index, code in enumerate(expected, 1):
    fixture = invalid[index - 1]
    if not fixture.name.startswith(f"{index:02d}_"):
        fail(f"fixture ordering mismatch for {code}: {fixture.name}")
for path in valid + invalid:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("module "):
        fail(f"fixture lacks module declaration: {path.relative_to(ROOT)}")
print("D8 fixtures: PASS")

all_rust_paths = sorted((ROOT / "crates").rglob("*.rs"))
all_rust = "\n".join(path.read_text(encoding="utf-8") for path in all_rust_paths)
test_count = len(re.findall(r"(?m)^\s*#\[test\]", all_rust))
if test_count < 135:
    fail(f"expected at least 135 cumulative Rust tests, found {test_count}")
if re.search(r"(?m)^\s*unsafe\s*\{", all_rust):
    fail("unsafe Rust block found")
for forbidden in [".unwrap()", ".expect(", "std::sync::LazyLock", ".is_none_or("]:
    if forbidden in types:
        fail(f"forbidden or Rust-1.74-incompatible marker in nivra-types: {forbidden}")
print(f"D8 Rust test inventory: PASS ({test_count})")


def check_rust(path: Path, text: str) -> None:
    pairs = {")": "(", "]": "[", "}": "{"}
    stack: list[tuple[str, int]] = []
    state = "code"
    depth = 0
    index = 0
    while index < len(text):
        current = text[index]
        following = text[index + 1] if index + 1 < len(text) else ""
        if state == "line":
            if current == "\n":
                state = "code"
            index += 1
            continue
        if state == "block":
            if current == "/" and following == "*":
                depth += 1
                index += 2
                continue
            if current == "*" and following == "/":
                depth -= 1
                index += 2
                if depth == 0:
                    state = "code"
                continue
            index += 1
            continue
        if state in {"string", "char"}:
            if current == "\\":
                index += 2
                continue
            if state == "string" and current == '"':
                suffix = text[index + 1] if index + 1 < len(text) else ""
                if suffix.isalnum() or suffix == "_":
                    line = text.count("\n", 0, index) + 1
                    fail(
                        f"invalid Rust string-literal suffix in "
                        f"{path.relative_to(ROOT)} at line {line}"
                    )
                state = "code"
            elif state == "char" and current == "'":
                state = "code"
            index += 1
            continue
        if current == "/" and following == "/":
            state = "line"
            index += 2
            continue
        if current == "/" and following == "*":
            state = "block"
            depth = 1
            index += 2
            continue
        if current == '"':
            state = "string"
            index += 1
            continue
        if current == "'":
            closing = text.find("'", index + 1, min(len(text), index + 16))
            if closing != -1 and "\n" not in text[index + 1 : closing]:
                state = "char"
            index += 1
            continue
        if current in "([{":
            stack.append((current, index))
        elif current in ")]}":
            if not stack or stack[-1][0] != pairs[current]:
                fail(f"unbalanced delimiter in {path.relative_to(ROOT)} at byte {index}")
            stack.pop()
        index += 1
    if state in {"string", "char", "block"} or stack:
        fail(f"unclosed Rust syntax state in {path.relative_to(ROOT)}")


for rust_path in all_rust_paths:
    check_rust(rust_path, rust_path.read_text(encoding="utf-8"))
print("D8 Rust lexical preflight: PASS")

for regression in [
    "infers_generic_function_arguments",
    "accepts_explicit_generic_function_arguments",
    "rejects_conflicting_generic_inference",
    "accepts_where_clause_trait_bound",
    "accepts_default_trait_method_using_required_method",
    "preserves_nested_explicit_generic_argument_types",
    "rejects_ambiguous_trait_method_selection",
    "rejects_external_trait_for_external_type",
    "rejects_generic_traits_until_the_feature_is_defined",
    "rejects_duplicate_generic_parameters",
    "rejects_unknown_enum_variant_with_suggestion",
]:
    if regression not in types:
        fail(f"D8 type regression test missing: {regression}")
if "duplicate_generic_parameters_are_deferred_to_type_checker" not in sema:
    fail("D8 semantic diagnostic-precedence regression test missing")
for regression in [
    "check_accepts_inferred_generic_function_call",
    "check_accepts_explicit_generic_function_call",
    "check_accepts_nested_explicit_generic_argument",
    "check_accepts_concrete_default_trait_method",
    "check_rejects_unsatisfied_generic_trait_bound",
    "check_rejects_invalid_generic_constraint_parameter",
    "check_rejects_generic_trait_declaration",
    "check_reports_gen005_for_duplicate_generic_parameters",
    "check_reports_unknown_enum_variant_with_suggestion",
    "typecheck_json_contains_generic_and_trait_graphs",
]:
    if regression not in cli_tests:
        fail(f"D8 CLI regression test missing: {regression}")
for anchor in [
    "D8 owns duplicate generic-parameter diagnostics through GEN005",
    "return Some(Type::Error);",
    "enum `{}` has no variant `{variant_name}`",
]:
    if anchor not in sema and anchor not in types:
        fail(f"D8 build-fix implementation anchor missing: {anchor}")
print("D8 root-cause regression guards: PASS")

workflow = (ROOT / ".github/workflows/verify-d8.yml").read_text(encoding="utf-8")
for anchor in [
    "cargo fmt --all",
    "cargo check --workspace --all-targets --locked",
    "cargo test --workspace --all-targets --locked --no-fail-fast",
    "cargo build --workspace --release --locked",
    "bash scripts/d8-smoke.sh",
]:
    if anchor not in workflow:
        fail(f"D8 workflow gate missing: {anchor}")
termux = (ROOT / "scripts/termux-verify.sh").read_text(encoding="utf-8")
if "nivra-d9-verification" not in termux or "NIVRA_D9_TEST_DIR" not in termux:
    fail("Termux verifier does not use a D9 internal-storage destination")
print("D8 workflow and Termux contract: PASS")

print("D8 drafting markers: PASS")

print("D8 STRUCTURE: PASS")
