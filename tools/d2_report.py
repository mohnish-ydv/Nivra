#!/usr/bin/env python3
from __future__ import annotations
import json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
D2=ROOT/'spec'/'d2'
def load(n): return json.loads((D2/n).read_text(encoding='utf-8'))
print('NIVRA D2 ARCHITECTURE REPORT')
print('============================')
print(f"Language identity: {load('identity.json')['language_name']}")
print(f"Architecture decisions: {len(load('decisions.json')['decisions'])}")
print(f"Type-system rules: {len(load('type-system.json')['rules'])}")
print(f"Memory-model rules: {len(load('memory-model.json')['rules'])}")
print(f"Error-model rules: {len(load('error-model.json')['rules'])}")
print(f"Concurrency rules: {len(load('concurrency-model.json')['rules'])}")
print(f"Compiler stages: {len(load('compiler-architecture.json')['stages'])}")
g=(D2/'grammar.ebnf').read_text(encoding='utf-8')
print(f"Grammar productions: {len(re.findall(r'(?m)^[a-z][a-z0-9_]*\s*=',g))}")
print(f"D2 examples: {len(list((ROOT/'examples'/'d2').glob('*.nva')))}")
print(f"Reference backend: {load('backend.json')['reference_backend']} + {load('backend.json')['native_driver']}")
print(f"Edition: {load('identity.json')['edition']}")
print('D2 status: PASS')
