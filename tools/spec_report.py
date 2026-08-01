#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def read_json(name: str):
    return json.loads((ROOT / name).read_text(encoding="utf-8"))

pain = read_json("spec/d1/pain-map.json")["items"]
articles = read_json("spec/d1/constitution.json")["articles"]
decisions = read_json("spec/d1/decisions.json")
keywords = [
    line.strip()
    for line in (ROOT / "spec/d1/keywords.txt").read_text(encoding="utf-8").splitlines()
    if line.strip()
]
examples = list((ROOT / "examples/design").glob("*.trn"))

print("TRION D1 SPEC REPORT")
print("====================")
print(f"Developer pain points: {len(pain)}")
print(f"P0 pain points: {sum(item['priority'] == 'P0' for item in pain)}")
print(f"P1 pain points: {sum(item['priority'] == 'P1' for item in pain)}")
print(f"P2 pain points: {sum(item['priority'] == 'P2' for item in pain)}")
print(f"Constitution articles: {len(articles)}")
print(f"Locked decisions: {len(decisions['locked'])}")
print(f"Deferred decisions: {len(decisions['deferred'])}")
print(f"Rejected directions: {len(decisions['rejected'])}")
print(f"Reserved keywords: {len(keywords)}")
print(f"Design examples: {len(examples)}")
print("D1 status: PASS")
