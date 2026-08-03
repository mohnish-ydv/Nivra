# D9 Manual Verification

## Full Termux verification

```bash
pkg update -y
pkg install rust python git unzip -y
cd ~/storage/downloads/Nivra-D9-Ownership-Borrow-Foundation-GitHub-Ready
bash scripts/termux-verify.sh
```

The verifier copies the source into `~/nivra-d9-verification` so Cargo does not build on shared Android storage.

Expected final lines after a fully successful executable run:

```text
D1 regression: PASS
D2 regression: PASS
D3 regression: PASS
D4 regression: PASS
D5 regression: PASS
D6 regression: PASS
D7 regression: PASS
D8 regression: PASS
D9 structure: PASS
Rust compilation: PASS
Rust tests: PASS
D9 ownership CLI smoke tests: PASS
★★★★★ D9 GOLDEN BUILD
```

## CLI identity and doctor

```bash
cd ~/nivra-d9-verification
./target/debug/nivra --version
./target/debug/nivra doctor
```

Expected identity:

```text
nivra 0.9.0 (ownership and borrow checking D9)
```

Doctor must include:

```text
D9 status: OPERATIONAL
Copy/move classification: PASS
```

## Valid ownership tour

```bash
./target/debug/nivra check examples/d9/05_complete_ownership_tour.nva
./target/debug/nivra ownership examples/d9/05_complete_ownership_tour.nva \
  --bindings --events --drops
```

The check command must exit 0. The ownership report must show binding classes/states, move/borrow events, and defer/drop actions.

## JSON ownership graph

```bash
./target/debug/nivra ownership \
  examples/d9/05_complete_ownership_tour.nva --json \
  | python3 -m json.tool >/dev/null
echo $?
```

Expected exit code: `0`.

## Diagnostic fixture matrix

```bash
codes=(OWN001 OWN002 OWN006 OWN007 BOR001 BOR002 BOR003 BOR004 BOR005 BOR006 BOR007 BOR008 BOR009)
i=0
for file in examples/d9/invalid/*.nva; do
  expected="${codes[$i]}"
  echo "== $file -> $expected =="
  output="$(./target/debug/nivra check "$file" 2>&1 || true)"
  printf '%s\n' "$output" | grep -F "$expected"
  i=$((i + 1))
done
```

Every fixture must contain its expected code and the original `nivra check` invocation must exit 1.

## Focused hardening regressions

```bash
cargo test -p nivra-ownership concrete_generic_copy_fields_make_the_nominal_copy --locked
cargo test -p nivra-ownership deferred_borrow_keeps_owner_live_until_scope_exit --locked
cargo test -p nivra-ownership mutable_reference_is_move_only_but_has_no_drop_action --locked
cargo test -p nivra-ownership rejects_returning_a_local_borrow_through_an_alias --locked
cargo test -p nivra-ownership rejects_tail_return_of_a_local_borrow_alias --locked
cargo test -p nivra-ownership rejects_borrowed_enum_variant_payloads --locked
```

Each command must report one passed test and zero failed tests.

## Fresh archive verification

```bash
bash scripts/fresh-extract-verify.sh
```

With Cargo available, this checks metadata, all targets, and the complete suite from a newly created and extracted archive. Without Cargo it reports that Rust execution was skipped and only labels the static fresh-extraction checks as PASS; it does not print the golden executable marker.

## Checkout versus release hygiene

Run `python3 tools/d9_hygiene_regression.py`. The command must report
`D9 repository/release hygiene regression: PASS`. A normal Git checkout may
contain `.git` and local build caches; a freshly extracted source release may not.
