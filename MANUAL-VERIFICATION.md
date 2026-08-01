# D5 Manual Verification After GitHub Actions Is Green

## 1. Install requirements

```bash
pkg update -y
pkg install rust python git -y
```

## 2. Run the Termux-safe golden verifier

```bash
cd ~/storage/downloads/Nivra-D5-Semantic-Resolution-GitHub-Ready
bash scripts/termux-verify.sh
```

The project is copied to `~/nivra-d5-verification` before Cargo runs.

Expected ending:

```text
D1 regression: PASS
D2 regression: PASS
D3 regression: PASS
D4 regression: PASS
D5 structure: PASS
Rust tests: PASS
D5 CLI smoke tests: PASS
★★★★★ D5 GOLDEN BUILD
```

## 3. Check the CLI identity

```bash
cd ~/nivra-d5-verification
./target/debug/nivra --version
./target/debug/nivra doctor
```

Expected version:

```text
nivra 0.5.0 (semantic name-resolution foundation D5)
```

## 4. Check a valid semantic program

```bash
./target/debug/nivra check examples/d5/05_complete_semantic_tour.nva
```

Expected ending contains:

```text
0 errors
```

## 5. Inspect symbols and scopes

```bash
./target/debug/nivra resolve \
  examples/d5/05_complete_semantic_tour.nva \
  --symbols --scopes | sed -n '1,220p'
```

Confirm that the output contains module, function, parameter, local, block, match-arm, closure,
task-group, and imported names.

## 6. Test unresolved-name diagnostics

```bash
./target/debug/nivra check examples/d5/invalid/03_unresolved_name.nva
echo $?
```

Expected:

```text
error[SEM003]
1
```

## 7. Test duplicate diagnostics

```bash
./target/debug/nivra check examples/d5/invalid/02_duplicate_local.nva
echo $?
```

Expected:

```text
error[SEM002]
1
```

## 8. Decode semantic JSON

```bash
./target/debug/nivra resolve \
  examples/d5/01_module_index.nva \
  --json | python3 -m json.tool >/dev/null

echo $?
```

Expected: `0`.

When all checks pass, report:

```text
GG D5 Passed
```
