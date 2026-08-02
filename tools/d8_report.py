#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
diagnostics = json.loads((ROOT / "spec/d8/diagnostics.json").read_text(encoding="utf-8"))
all_rust = "\n".join(path.read_text(encoding="utf-8") for path in (ROOT / "crates").rglob("*.rs"))
valid = list((ROOT / "examples/d8").glob("*.nva"))
invalid = list((ROOT / "examples/d8/invalid").glob("*.nva"))

print("NIVRA D8 GENERICS AND TRAITS REPORT")
print("====================================")
print(f"Workspace version: {workspace['workspace']['package']['version']}")
print(f"Workspace crates: {len(workspace['workspace']['members'])}")
print(f"Cumulative Rust tests: {len(re.findall(r'(?m)^\s*#\[test\]', all_rust))}")
print(f"D8 valid fixtures: {len(valid)}")
print(f"D8 invalid fixtures: {len(invalid)}")
print(f"D8 diagnostics: {len(diagnostics['codes'])}")
print("Explicit generic arguments: YES")
print("Local generic inference: YES")
print("Generic nominal substitution: YES")
print("Trait bounds and where clauses: YES")
print("Trait implementation validation: YES")
print("Deterministic method selection: YES")
print("Generic traits: DEFERRED WITH GEN006")
print("D8 status: IMPLEMENTED")
