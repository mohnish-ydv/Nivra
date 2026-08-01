# D6 Manual Verification After GitHub Actions Is Green

## 1. Run the Termux-safe cumulative verifier

```bash
cd ~/storage/downloads/Nivra-D6-Type-Checker-GitHub-Ready
bash scripts/termux-verify.sh
```

The project is copied to `~/nivra-d6-verification` before Rust compilation.
Expected ending:

```text
D1 regression: PASS
D2 regression: PASS
D3 regression: PASS
D4 regression: PASS
D5 regression: PASS
D6 structure: PASS
Rust tests: PASS
D6 CLI smoke tests: PASS
★★★★★ D6 GOLDEN BUILD
```

## 2. Confirm CLI identity

```bash
cd ~/nivra-d6-verification
./target/debug/nivra --version
./target/debug/nivra doctor
```

Expected identity:

```text
nivra 0.6.0 (static type-checker foundation D6)
D6 status: OPERATIONAL
```

## 3. Check the complete valid program

```bash
./target/debug/nivra check examples/d6/05_complete_type_tour.nva
```

Expected ending contains `0 errors`.

## 4. Inspect signatures and inferred bindings

```bash
./target/debug/nivra typecheck \
  examples/d6/05_complete_type_tour.nva \
  --functions --types | sed -n '1,240p'
```

Confirm that `clamp`, `score`, `bounded`, `final_score`, `tags`, `contact`, and
`attempts` have readable types.

## 5. Check representative failures

```bash
./target/debug/nivra typecheck examples/d6/invalid/01_binding_mismatch.nva
echo $?
./target/debug/nivra typecheck examples/d6/invalid/03_wrong_arity.nva
echo $?
./target/debug/nivra typecheck examples/d6/invalid/07_non_bool_condition.nva
echo $?
./target/debug/nivra typecheck examples/d6/invalid/10_immutable_assignment.nva
echo $?
```

Expected codes are `TYP001`, `TYP003`, `TYP007`, and `TYP010`; each command exits
with code `1`.

## 6. Validate JSON output

```bash
./target/debug/nivra typecheck \
  examples/d6/02_functions_and_calls.nva \
  --json | python3 -m json.tool >/dev/null

echo $?
```

Expected exit code: `0`.

## Pass message

When Actions and every manual check pass, report:

```text
GG D6 Passed
```
