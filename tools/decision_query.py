#!/usr/bin/env python3
from __future__ import annotations
import json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
D2=ROOT/'spec'/'d2'
areas={
 'memory':'memory-model.json','types':'type-system.json','errors':'error-model.json',
 'concurrency':'concurrency-model.json','compiler':'compiler-architecture.json',
 'backend':'backend.json','identity':'identity.json','packages':'package-model.json','ffi':'ffi.json'
}
key=sys.argv[1].lower() if len(sys.argv)>1 else 'memory'
if key not in areas:
    print('Available: ' + ', '.join(sorted(areas)))
    raise SystemExit(2)
data=json.loads((D2/areas[key]).read_text(encoding='utf-8'))
print(f'NIVRA D2 DECISION QUERY: {key.upper()}')
print('='*(25+len(key)))
if 'rules' in data:
    for row in data['rules']:
        print(f"{row['id']}: {row['rule']}")
else:
    for k,v in data.items():
        if k in {'schema_version','delivery'}: continue
        print(f'{k}: {v}')
