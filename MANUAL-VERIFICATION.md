# Manual Verification After GitHub Actions Is Green

D2 is still a design delivery, so the manual test checks architecture integrity
rather than compiling `.nva` programs.

## 1. Extract and enter the project

```bash
cd ~/storage/downloads
unzip Nivra-D2-Architecture-Spec-GitHub-Ready.zip
cd Nivra-D2-Architecture-Spec-GitHub-Ready
```

## 2. Run the cumulative golden verifier

```bash
bash verify.sh
```

Expected ending:

```text
D1 regression: PASS
D2 architecture integrity: PASS
D2 grammar integrity: PASS
D2 examples: PASS
★★★★★ D2 GOLDEN BUILD
```

## 3. Print the architecture report

```bash
python3 tools/d2_report.py
```

Expected key values:

```text
Architecture decisions: 45
Type-system rules: 29
Memory-model rules: 24
Error-model rules: 18
Concurrency rules: 24
Compiler stages: 13
Grammar productions: 60
D2 examples: 8
D2 status: PASS
```

## 4. Query the locked memory model

```bash
python3 tools/decision_query.py memory
```

Confirm it reports:

- deterministic destruction
- move-by-default for non-copy values
- `Box<T>`, `Shared<T>`, and `Weak<T>`
- local borrows through `&T` and `&mut T`
- no borrowed fields in Edition 2026
- no borrow crossing an `await`
- no mandatory tracing garbage collector
- named unsafe capabilities

## 5. Read the complete architecture syntax tour

```bash
sed -n '1,300p' examples/d2/08_complete_architecture_tour.nva
```

Check that:

- `.nva` and Nivra naming are used
- `Unit`, not `Void`, represents no useful value
- integer overflow is checked by default
- fallible work returns `Result`
- non-copy values move unless explicitly cloned
- shared ownership is explicit
- task creation occurs inside a `task_group`
- FFI and raw memory are inside named unsafe capabilities

## 6. Read the specification index

```bash
sed -n '1,320p' docs/16-LANGUAGE-SPEC-DRAFT.md
```

The document must clearly separate normative Edition 2026 rules from future
extensions.

## Pass rule

When GitHub Actions and all checks above pass, report:

```text
GG D2 Passed
```
