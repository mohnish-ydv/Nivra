#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import sys
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
        fail(f"Invalid JSON in {relative}: {exc}")

required_files = [
    "README.md",
    "DELIVERY-REPORT.md",
    "ACCEPTANCE-CHECKLIST.md",
    "MANUAL-VERIFICATION.md",
    "docs/00-MISSION.md",
    "docs/01-DEVELOPER-PAIN-MAP.md",
    "docs/02-LANGUAGE-CONSTITUTION.md",
    "docs/03-SYNTAX-DIRECTION.md",
    "docs/04-NON-GOALS.md",
    "docs/DECISION-SUMMARY.md",
    "spec/d1/mission.json",
    "spec/d1/pain-map.json",
    "spec/d1/constitution.json",
    "spec/d1/decisions.json",
    "spec/d1/syntax-profile.json",
    "spec/d1/keywords.txt",
]
missing = [item for item in required_files if not (ROOT / item).is_file()]
if missing:
    fail("Missing required files: " + ", ".join(missing))
print("Required files: PASS")

mission = load_json("spec/d1/mission.json")
pain = load_json("spec/d1/pain-map.json")
constitution = load_json("spec/d1/constitution.json")
decisions = load_json("spec/d1/decisions.json")
syntax = load_json("spec/d1/syntax-profile.json")
print("JSON parsing: PASS")

items = pain.get("items", [])
if len(items) < 25:
    fail(f"Expected at least 25 pain points, found {len(items)}")
pain_ids = [item["id"] for item in items]
if len(pain_ids) != len(set(pain_ids)):
    fail("Duplicate pain IDs")
required_p0_categories = {
    "build", "packages", "diagnostics", "safety", "errors",
    "concurrency", "tooling", "reproducibility", "types"
}
p0_categories = {item["category"] for item in items if item["priority"] == "P0"}
missing_categories = sorted(required_p0_categories - p0_categories)
if missing_categories:
    fail("Missing required P0 categories: " + ", ".join(missing_categories))
print("Pain map integrity: PASS")

articles = constitution.get("articles", [])
if len(articles) < 15:
    fail(f"Expected at least 15 constitution articles, found {len(articles)}")
article_ids = [article["id"] for article in articles]
if len(article_ids) != len(set(article_ids)):
    fail("Duplicate constitution article IDs")
if any(article.get("status") != "locked" for article in articles):
    fail("Every D1 constitution article must be locked")
print("Constitution integrity: PASS")

locked = set(decisions.get("locked", []))
deferred = set(decisions.get("deferred", []))
rejected = set(decisions.get("rejected", []))
if locked & deferred or locked & rejected or deferred & rejected:
    fail("Decision sets overlap")
if decisions.get("working_identity", {}).get("status") != "provisional":
    fail("Working identity must remain provisional in D1")
print("Decision separation: PASS")

required_syntax = {
    "block_delimiter": "braces",
    "indentation_semantic": False,
    "line_semicolon_required": False,
    "immutable_binding": "let",
    "mutable_binding": "var",
    "absence_literal": "none",
    "general_truthiness": False,
    "class_inheritance_v1": False,
}
for key, expected in required_syntax.items():
    if syntax.get(key) != expected:
        fail(f"Syntax invariant {key!r} expected {expected!r}, got {syntax.get(key)!r}")
print("Syntax profile invariants: PASS")

keywords = [
    line.strip()
    for line in (ROOT / "spec/d1/keywords.txt").read_text(encoding="utf-8").splitlines()
    if line.strip()
]
if len(keywords) != len(set(keywords)):
    fail("Duplicate reserved keywords")
if len(keywords) < 35:
    fail(f"Expected at least 35 keywords, found {len(keywords)}")
print("Keyword uniqueness: PASS")

def balanced(source: str, path: Path) -> None:
    pairs = {")": "(", "]": "[", "}": "{"}
    stack = []
    in_string = False
    in_char = False
    escaped = False
    i = 0
    while i < len(source):
        ch = source[i]
        nxt = source[i + 1] if i + 1 < len(source) else ""
        if escaped:
            escaped = False
            i += 1
            continue
        if (in_string or in_char) and ch == "\\":
            escaped = True
            i += 1
            continue
        if not in_char and ch == '"':
            in_string = not in_string
            i += 1
            continue
        if not in_string and ch == "'":
            in_char = not in_char
            i += 1
            continue
        if in_string or in_char:
            i += 1
            continue
        if ch == "/" and nxt == "/":
            nl = source.find("\n", i)
            if nl == -1:
                break
            i = nl + 1
            continue
        if ch in "([{":
            stack.append(ch)
        elif ch in ")]}":
            if not stack or stack.pop() != pairs[ch]:
                fail(f"Unbalanced delimiter in {path.relative_to(ROOT)}")
        i += 1
    if in_string or in_char or stack:
        fail(f"Unclosed string/character/delimiter in {path.relative_to(ROOT)}")

examples = sorted((ROOT / "examples/design").glob("*.trn"))
if len(examples) < 5:
    fail(f"Expected at least 5 design examples, found {len(examples)}")
for path in examples:
    text = path.read_text(encoding="utf-8")
    if "module " not in text:
        fail(f"Missing module declaration in {path.name}")
    balanced(text, path)
if "fn main()" not in (ROOT / "examples/design/01_hello.trn").read_text(encoding="utf-8"):
    fail("Hello example has no main function")
print("Design examples: PASS")

placeholder_pattern = re.compile(r"\b(TODO|TBD|FIXME)\b")
for path in ROOT.rglob("*"):
    if not path.is_file() or ".git" in path.parts:
        continue
    if path.suffix.lower() not in {".md", ".json", ".txt", ".trn", ".py", ".sh", ".yml", ".yaml"}:
        continue
    if path.name == "spec_lint.py":
        continue
    text = path.read_text(encoding="utf-8")
    if placeholder_pattern.search(text):
        fail(f"Unresolved placeholder found in {path.relative_to(ROOT)}")
print("No unresolved placeholders: PASS")

anchors = {
    "docs/00-MISSION.md": ["## Core promise", "## Success metrics", "## Anti-goals"],
    "docs/01-DEVELOPER-PAIN-MAP.md": ["## P0", "## P1", "## P2"],
    "docs/02-LANGUAGE-CONSTITUTION.md": ["## Article C-001", "## Article C-018"],
    "docs/03-SYNTAX-DIRECTION.md": ["## Bindings", "## Recoverable errors", "## Unsafe boundary"],
    "docs/DECISION-SUMMARY.md": ["## Locked in D1", "## Deferred to D2", "## Rejected for V1"],
}
for rel, required in anchors.items():
    text = (ROOT / rel).read_text(encoding="utf-8")
    for anchor in required:
        if anchor not in text:
            fail(f"Missing anchor {anchor!r} in {rel}")
print("Documentation anchors: PASS")

if mission.get("delivery") != "D1":
    fail("Mission delivery must be D1")
print("Specification integrity: PASS")
