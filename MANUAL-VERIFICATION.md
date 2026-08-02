# D7 Manual Verification After GitHub Actions Is Green

## 1. Install phone requirements

```bash
pkg update -y
pkg install rust python git unzip -y
```

## 2. Run complete Termux-safe verification

```bash
cd ~/storage/downloads/Nivra-D7-Nominal-Members-Build-Fix-GitHub-Ready
bash scripts/termux-verify.sh
```

The project is copied to:

```text
~/nivra-d7-verification
```

Expected ending:

```text
D1 regression: PASS
D2 regression: PASS
D3 regression: PASS
D4 regression: PASS
D5 regression: PASS
D6 regression: PASS
D7 structure: PASS
Rust tests: PASS
D7 CLI smoke tests: PASS
★★★★★ D7 GOLDEN BUILD
```

## 3. Check CLI identity

```bash
cd ~/nivra-d7-verification
./target/debug/nivra --version
./target/debug/nivra doctor
```

Expected:

```text
nivra 0.7.0 (nominal types and members D7)
D7 status: OPERATIONAL
```

## 4. Check the complete valid tour

```bash
./target/debug/nivra check \
  examples/d7/05_complete_nominal_tour.nva
```

Expected ending:

```text
0 errors
```

## 5. Inspect nominal types and members

```bash
./target/debug/nivra typecheck \
  examples/d7/05_complete_nominal_tour.nva \
  --functions --types --nominals
```

Confirm the report includes:

```text
record Profile
field name: String
field score: Int = <default>
method add_score
method is_active
variant online(Profile)
```

## 6. Inspect lossless record-construction CST

```bash
./target/debug/nivra parse \
  examples/d7/01_records_and_construction.nva \
  --tree | grep -E 'record_expression|record_field_initializer'
```

Both node names must appear.

## 7. Check representative diagnostics

```bash
./target/debug/nivra typecheck \
  examples/d7/invalid/01_unknown_member.nva
echo $?

./target/debug/nivra typecheck \
  examples/d7/invalid/03_missing_required_field.nva
echo $?

./target/debug/nivra typecheck \
  examples/d7/invalid/07_enum_variant_payload.nva
echo $?

./target/debug/nivra typecheck \
  examples/d7/invalid/08_immutable_member_mutation.nva
echo $?
```

Expected codes and exit status:

```text
NOM001 → 1
NOM003 → 1
NOM007 → 1
NOM008 → 1
```

## 8. Validate JSON nominal graph

```bash
./target/debug/nivra typecheck \
  examples/d7/05_complete_nominal_tour.nva \
  --json | python3 -m json.tool >/dev/null

echo $?
```

Expected:

```text
0
```

## Pass message

After all checks pass:

```text
GG D7 Passed
```
