# D8 Manual Verification

Run these checks only after GitHub Actions shows a green result for
`Verify D8 Generics and Traits`.

## Full Termux verification

```bash
cd ~/storage/downloads/Nivra-D8-Generics-Traits-GitHub-Ready
bash scripts/termux-verify.sh
```

The project is copied to `~/nivra-d8-verification` before Cargo runs.

Expected ending:

```text
D1 regression: PASS
D2 regression: PASS
D3 regression: PASS
D4 regression: PASS
D5 regression: PASS
D6 regression: PASS
D7 regression: PASS
D8 structure: PASS
Rust compilation: PASS
Rust tests: PASS
D8 CLI smoke tests: PASS
★★★★★ D8 GOLDEN BUILD
```

## CLI identity

```bash
cd ~/nivra-d8-verification
./target/debug/nivra --version
./target/debug/nivra doctor
```

Expected version:

```text
nivra 0.8.0 (generics and traits D8)
```

## Complete valid tour

```bash
./target/debug/nivra check examples/d8/05_complete_generics_traits_tour.nva
```

Expected ending: `0 errors`.

## Generic and trait report

```bash
./target/debug/nivra typecheck \
  examples/d8/05_complete_generics_traits_tour.nva \
  --functions --types --nominals --traits | sed -n '1,260p'
```

Confirm that `identity<T>`, `Box<T>`, `Display`, and its implementations appear.

## Representative diagnostics

```bash
./target/debug/nivra check examples/d8/invalid/01_wrong_generic_arity.nva; echo $?
./target/debug/nivra check examples/d8/invalid/04_unsatisfied_bound.nva; echo $?
./target/debug/nivra check examples/d8/invalid/06_generic_trait_deferred.nva; echo $?
./target/debug/nivra check examples/d8/invalid/09_missing_trait_method.nva; echo $?
./target/debug/nivra check examples/d8/invalid/11_ambiguous_trait_method.nva; echo $?
./target/debug/nivra check examples/d8/invalid/12_orphan_rule.nva; echo $?
```

Expected codes respectively: GEN001, GEN004, GEN006, TRT003, TRT005, TRT006.
Each command must exit with code 1.

## JSON graph

```bash
./target/debug/nivra typecheck \
  examples/d8/05_complete_generics_traits_tour.nva \
  --json | python3 -m json.tool >/dev/null
echo $?
```

Expected: `0`.
