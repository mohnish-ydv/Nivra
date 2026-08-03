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
    "crates/nivra-ownership/Cargo.toml",
    "crates/nivra-ownership/src/lib.rs",
    "spec/d9/delivery.json",
    "spec/d9/ownership-model.json",
    "spec/d9/borrow-model.json",
    "spec/d9/drop-model.json",
    "spec/d9/diagnostics.json",
    "spec/d9/checker-pipeline.json",
    "docs/56-D9-IMPLEMENTATION.md",
    "docs/57-COPY-MOVE-AND-PARTIAL-MOVES.md",
    "docs/58-BORROW-REGION-INFERENCE.md",
    "docs/59-DETERMINISTIC-DROP-PLANNING.md",
    "docs/60-D9-DIAGNOSTICS.md",
    "docs/61-D9-TO-D10-GATE.md",
    "D9-IMPLEMENTATION-REPORT.md",
    "D9-QA-REPORT.md",
    "RELEASE-NOTES-D9.md",
    "scripts/d9-smoke.sh",
    "tools/d9_report.py",
    ".github/workflows/verify-d9.yml",
]
missing = [item for item in required if not (ROOT / item).is_file()]
if missing:
    fail("missing D9 files: " + ", ".join(missing))
print("D9 required files: PASS")

delivery = load_json("spec/d9/delivery.json")
ownership_model = load_json("spec/d9/ownership-model.json")
borrow_model = load_json("spec/d9/borrow-model.json")
drop_model = load_json("spec/d9/drop-model.json")
diagnostics = load_json("spec/d9/diagnostics.json")
pipeline = load_json("spec/d9/checker-pipeline.json")
if delivery.get("delivery") != "D9" or delivery.get("version") != "0.9.0":
    fail("D9 delivery identity mismatch")
if delivery.get("workspace_crates") != 9 or delivery.get("new_crate") != "nivra-ownership":
    fail("D9 workspace metadata mismatch")
if ownership_model.get("transfer_classes") != ["copy", "move"]:
    fail("D9 transfer classes mismatch")
if borrow_model.get("user_written_lifetimes") is not False:
    fail("Edition 2026 lifetime policy changed")
if drop_model.get("scope_exit_order", [None])[0] != "defer actions in reverse registration order":
    fail("defer/drop order metadata mismatch")
if pipeline.get("ownership_runs_after_successful_typecheck") is not True:
    fail("ownership pipeline ordering mismatch")
print("D9 metadata: PASS")

workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
package = workspace["workspace"]["package"]
if package.get("version") != "0.9.0":
    fail("workspace version is not 0.9.0")
if package.get("rust-version") != "1.74":
    fail("Rust version policy changed")
members = workspace["workspace"]["members"]
if len(members) != 9 or "crates/nivra-ownership" not in members:
    fail(f"D9 should contain nine focused crates, found {len(members)}")
lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
if lock.count('version = "0.9.0"') != 9:
    fail("Cargo.lock does not contain nine 0.9.0 workspace packages")
if "registry+" in lock or "checksum =" in lock:
    fail("D9 unexpectedly introduced registry dependencies")
print("D9 Cargo workspace: PASS")

parser = (ROOT / "crates/nivra-parser/src/lib.rs").read_text(encoding="utf-8")
types = (ROOT / "crates/nivra-types/src/lib.rs").read_text(encoding="utf-8")
ownership = (ROOT / "crates/nivra-ownership/src/lib.rs").read_text(encoding="utf-8")
cli = (ROOT / "crates/nivra-cli/src/main.rs").read_text(encoding="utf-8")
cli_tests = (ROOT / "crates/nivra-cli/tests/cli.rs").read_text(encoding="utf-8")
for anchor in [
    "Keyword::Move",
    "parses_explicit_move_prefix_expression",
]:
    if anchor not in parser:
        fail(f"D9 parser anchor missing: {anchor}")
for anchor in [
    "TokenKind::Keyword(Keyword::Move) => operand",
    "explicit_move_preserves_operand_static_type",
]:
    if anchor not in types:
        fail(f"D9 type anchor missing: {anchor}")
for anchor in [
    "pub enum OwnershipClass",
    "pub enum ValueState",
    "pub enum BorrowKind",
    "pub struct OwnershipResult",
    "pub fn analyze(",
    "pub fn classify_type(",
    "fn use_place(",
    "fn borrow_place(",
    "fn merge_snapshots(",
    "borrow_scope_id",
    "explicit_move_invalidates_even_outside_a_consuming_call",
    "both_moving_branches_join_as_moved_not_maybe_moved",
    "inner_scope_borrow_of_outer_owner_ends_with_reference_scope",
    "concrete_generic_copy_fields_make_the_nominal_copy",
    "mutable_reference_is_move_only_but_has_no_drop_action",
    "deferred_borrow_keeps_owner_live_until_scope_exit",
    "rejects_returning_a_local_borrow_through_an_alias",
    "rejects_tail_return_of_a_local_borrow_alias",
    "rejects_borrowed_enum_variant_payloads",
]:
    if anchor not in ownership:
        fail(f"D9 ownership anchor missing: {anchor}")
for anchor in [
    '"ownership" => ownership_command',
    "fn parse_ownership_options(",
    "fn ownership_json(",
    "Copy/move classification: PASS",
    "D9 status: OPERATIONAL",
]:
    if anchor not in cli:
        fail(f"D9 CLI anchor missing: {anchor}")
print("D9 implementation anchors: PASS")

codes = [item["code"] for item in diagnostics.get("codes", [])]
expected = ["OWN001", "OWN002", "OWN006", "OWN007"] + [
    f"BOR{number:03d}" for number in range(1, 10)
]
if codes != expected or len(codes) != len(set(codes)):
    fail(f"D9 diagnostic inventory mismatch: {codes}")
for code in expected:
    if code not in ownership or code not in cli:
        fail(f"{code} is not implemented and explained")
print("D9 diagnostics: PASS")

valid = sorted((ROOT / "examples/d9").glob("*.nva"))
invalid = sorted((ROOT / "examples/d9/invalid").glob("*.nva"))
if len(valid) != 5 or len(invalid) != len(expected):
    fail(f"expected 5 valid and {len(expected)} invalid D9 fixtures, found {len(valid)} and {len(invalid)}")
for index, (fixture, code) in enumerate(zip(invalid, expected, strict=True), 1):
    if not fixture.name.startswith(f"{index:02d}_"):
        fail(f"fixture ordering mismatch for {code}: {fixture.name}")
for path in valid + invalid:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("module "):
        fail(f"fixture lacks module declaration: {path.relative_to(ROOT)}")
print("D9 fixtures: PASS")

all_rust_paths = sorted((ROOT / "crates").rglob("*.rs"))
all_rust = "\n".join(path.read_text(encoding="utf-8") for path in all_rust_paths)
test_count = len(re.findall(r"(?m)^\s*#\[test\]", all_rust))
if test_count < 155:
    fail(f"expected at least 155 cumulative Rust tests, found {test_count}")
if re.search(r"(?m)^\s*unsafe\s*\{", all_rust):
    fail("unsafe Rust block found")
for forbidden in [".unwrap()", ".expect(", "std::sync::LazyLock", ".is_none_or("]:
    if forbidden in ownership:
        fail(f"forbidden or Rust-1.74-incompatible marker in nivra-ownership: {forbidden}")
print(f"D9 Rust test inventory: PASS ({test_count})")


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
                    fail(f"invalid Rust string-literal suffix in {path.relative_to(ROOT)} at line {line}")
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
print("D9 Rust lexical preflight: PASS")

for regression in [
    "rejects_use_after_move",
    "accepts_copy_after_transfer",
    "rejects_shared_then_mutable_borrow_conflict",
    "last_use_ends_local_borrow_without_lifetime_syntax",
    "mutable_borrow_requires_var",
    "moved_var_can_be_reinitialized",
    "plans_defers_before_reverse_local_drops",
    "rejects_borrow_across_await",
    "explicit_move_invalidates_even_outside_a_consuming_call",
    "both_moving_branches_join_as_moved_not_maybe_moved",
    "move_on_only_one_branch_is_maybe_moved",
    "inner_scope_borrow_of_outer_owner_ends_with_reference_scope",
    "rejects_complete_use_after_partial_field_move",
]:
    if regression not in ownership:
        fail(f"D9 ownership regression test missing: {regression}")
for regression in [
    "version_reports_d9_foundation",
    "ownership_command_reports_moves_borrows_and_drops",
    "ownership_json_contains_machine_readable_flow_graph",
    "check_reports_use_after_move_from_ownership_phase",
    "check_accepts_explicit_move_and_rejects_source_reuse",
    "explain_supports_d9_ownership_and_borrow_diagnostics",
]:
    if regression not in cli_tests:
        fail(f"D9 CLI regression test missing: {regression}")
print("D9 root-cause regression guards: PASS")

workflow = (ROOT / ".github/workflows/verify-d9.yml").read_text(encoding="utf-8")
for anchor in [
    "cargo fmt --all -- --check",
    "cargo check --workspace --all-targets --locked",
    "cargo test --workspace --all-targets --locked --no-fail-fast",
    "cargo build --workspace --release --locked",
    "bash scripts/d9-smoke.sh",
    "fresh-extract",
]:
    if anchor not in workflow:
        fail(f"D9 workflow gate missing: {anchor}")
termux = (ROOT / "scripts/termux-verify.sh").read_text(encoding="utf-8")
if "nivra-d9-verification" not in termux or "NIVRA_D9_TEST_DIR" not in termux:
    fail("Termux verifier does not use a D9 internal-storage destination")
print("D9 workflow and Termux contract: PASS")

print("D9 STRUCTURE: PASS")
