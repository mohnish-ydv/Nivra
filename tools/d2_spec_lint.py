#!/usr/bin/env python3
from __future__ import annotations
import json, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
D2 = ROOT / 'spec' / 'd2'

def fail(msg: str) -> None:
    print(f'FAIL: {msg}')
    raise SystemExit(1)

def load(name: str):
    p = D2 / name
    try:
        return json.loads(p.read_text(encoding='utf-8'))
    except Exception as exc:
        fail(f'invalid JSON {name}: {exc}')

required = [
 'identity.json','type-system.json','memory-model.json','error-model.json',
 'concurrency-model.json','compiler-architecture.json','backend.json','ffi.json',
 'package-model.json','compatibility.json','decisions.json','keywords.txt','grammar.ebnf'
]
missing = [x for x in required if not (D2/x).is_file()]
if missing: fail('missing D2 files: ' + ', '.join(missing))
print('D2 required files: PASS')

identity=load('identity.json'); types=load('type-system.json'); memory=load('memory-model.json')
errors=load('error-model.json'); concurrency=load('concurrency-model.json')
compiler=load('compiler-architecture.json'); backend=load('backend.json'); ffi=load('ffi.json')
packages=load('package-model.json'); compat=load('compatibility.json'); decisions=load('decisions.json')
print('D2 JSON parsing: PASS')

expected_identity = {'language_name':'Nivra','cli':'nivra','source_extension':'.nva','manifest':'nivra.toml','lockfile':'nivra.lock','edition':'2026'}
for key,val in expected_identity.items():
    if identity.get(key) != val: fail(f'identity {key} expected {val!r}')
if identity.get('legal_clearance_claimed') is not False: fail('identity must not claim legal clearance')
print('D2 identity migration: PASS')

def unique_rules(doc, key='rules'):
    rows=doc[key]; ids=[x['id'] for x in rows]
    if len(ids) != len(set(ids)): fail('duplicate rule IDs')
    if any(x.get('status')!='locked' for x in rows): fail('all D2 rules must be locked')
    return rows

tr=unique_rules(types); mr=unique_rules(memory); er=unique_rules(errors); cr=unique_rules(concurrency)
if len(tr)!=29: fail(f'expected 29 type rules, found {len(tr)}')
if len(mr)!=24: fail(f'expected 24 memory rules, found {len(mr)}')
if len(er)!=18: fail(f'expected 18 error rules, found {len(er)}')
if len(cr)!=24: fail(f'expected 24 concurrency rules, found {len(cr)}')
print('D2 semantic rule counts: PASS')

stages=compiler['stages']; stage_ids=[x['id'] for x in stages]
if len(stages)!=13 or len(stage_ids)!=len(set(stage_ids)): fail('compiler stages invalid')
if compiler.get('implementation_language')!='Rust': fail('bootstrap language must be Rust')
if backend.get('reference_backend')!='C11' or backend.get('native_driver')!='Clang': fail('reference backend mismatch')
if backend.get('mir_backend_neutral') is not True: fail('MIR must be backend neutral')
print('D2 compiler and backend: PASS')

if ffi.get('v1_abi')!='C' or ffi.get('cpp_abi_promised') is not False: fail('FFI policy mismatch')
if packages.get('arbitrary_install_scripts') is not False: fail('install scripts must be rejected')
if compat.get('first_edition')!='2026' or compat.get('silent_semantic_change_existing_edition') is not False: fail('edition policy mismatch')
print('D2 FFI, package, compatibility: PASS')

dec=decisions['decisions']; ids=[x['id'] for x in dec]
if len(dec)!=45 or len(ids)!=len(set(ids)): fail('expected 45 unique architecture decisions')
locked={x['decision'] for x in dec}; deferred=set(decisions['deferred']); rejected=set(decisions['rejected'])
if locked & deferred or locked & rejected or deferred & rejected: fail('decision sets overlap')
print('D2 decision integrity: PASS')

keywords=[x.strip() for x in (D2/'keywords.txt').read_text(encoding='utf-8').splitlines() if x.strip()]
if len(keywords)!=len(set(keywords)): fail('duplicate D2 keywords')
for required_keyword in ['newtype','ensure','task_group','unsafe','await','dyn']:
    if required_keyword not in keywords: fail(f'missing keyword {required_keyword}')
print('D2 keyword integrity: PASS')

# Parse EBNF productions and validate references to grammar nonterminals.
gtext=(D2/'grammar.ebnf').read_text(encoding='utf-8')
production_re=re.compile(r'(?m)^([a-z][a-z0-9_]*)\s*=\s*(.*?)\s*;\s*$')
productions=production_re.findall(gtext)
if len(productions)!=60: fail(f'expected 60 grammar productions, found {len(productions)}')
names=[n for n,_ in productions]
if len(names)!=len(set(names)): fail('duplicate grammar production')
defined=set(names)
# Tokens intentionally provided by the lexer rather than grammar productions.
lexical={'identifier','newline','integer_literal','float_literal','string_literal','char_literal','argument_list','assignment_operator','logical_operator','comparison_operator','additive_operator','multiplicative_operator','unary_operator','call_suffix','member_suffix','index_suffix','await_suffix','try_suffix','tuple_expr','array_expr','if_expr','match_expr','loop_expr','closure_expr','task_group_expr','unsafe_expr','tuple_pattern','variant_pattern'}
for name,body in productions:
    body_no_strings=re.sub(r'"(?:\\.|[^"\\])*"',' ',body)
    refs=set(re.findall(r'\b[a-z][a-z0-9_]*\b',body_no_strings))
    undefined=refs-defined-lexical
    if undefined: fail(f'undefined grammar reference in {name}: {sorted(undefined)}')
print('D2 grammar integrity: PASS')

# Validate examples.
examples=sorted((ROOT/'examples'/'d2').glob('*.nva'))
if len(examples)!=8: fail(f'expected 8 D2 examples, found {len(examples)}')

def balanced(text,path):
    pairs={')':'(',']':'[','}':'{'}; stack=[]; quote=None; escaped=False; i=0
    while i<len(text):
        ch=text[i]; nxt=text[i+1] if i+1<len(text) else ''
        if escaped: escaped=False; i+=1; continue
        if quote and ch=='\\': escaped=True; i+=1; continue
        if quote:
            if ch==quote: quote=None
            i+=1; continue
        if ch in ('"', "'"): quote=ch; i+=1; continue
        if ch=='/' and nxt=='/':
            j=text.find('\n',i); i=len(text) if j<0 else j+1; continue
        if ch in '([{': stack.append(ch)
        elif ch in ')]}':
            if not stack or stack.pop()!=pairs[ch]: fail(f'unbalanced delimiter {path.name}')
        i+=1
    if stack or quote: fail(f'unclosed delimiter or string {path.name}')

for p in examples:
    text=p.read_text(encoding='utf-8')
    if 'module ' not in text: fail(f'missing module in {p.name}')
    if 'Trion' in text or '.trn' in text: fail(f'old identity in {p.name}')
    balanced(text,p)

tour=(ROOT/'examples/d2/08_complete_architecture_tour.nva').read_text(encoding='utf-8')
coverage=['newtype ','Result<','&mut ','Shared<','task_group ','await ','extern "C"','unsafe(ffi, memory)','Unit','checked_add']
for item in coverage:
    if item not in tour: fail(f'complete tour missing {item!r}')
print('D2 examples: PASS')

# Contradiction scan over current normative D2 docs.
normative=[ROOT/'docs'/f for f in ['07-TYPE-SYSTEM.md','08-MEMORY-MODEL.md','09-ERROR-MODEL.md','10-CONCURRENCY-MODEL.md','11-COMPILER-ARCHITECTURE.md','12-BACKEND-AND-PORTABILITY.md','13-ABI-AND-FFI.md','14-PACKAGE-AND-BUILD-MODEL.md','15-COMPATIBILITY-AND-EDITIONS.md','16-LANGUAGE-SPEC-DRAFT.md']]
combined='\n'.join(p.read_text(encoding='utf-8') for p in normative)
required_phrases=['no mandatory tracing garbage collector','panic aborts','no `null`','C ABI','C11','Rust','no borrow crossing','structured']
for phrase in required_phrases:
    if phrase.lower() not in combined.lower(): fail(f'normative docs missing invariant phrase {phrase!r}')
for forbidden in ['mandatory tracing garbage collector by default','panic is catchable','implicit detached tasks are allowed','release builds wrap integer overflow silently']:
    if forbidden.lower() in combined.lower(): fail(f'forbidden semantic contradiction: {forbidden}')
print('D2 semantic contradictions: PASS')

print('D2 architecture integrity: PASS')
