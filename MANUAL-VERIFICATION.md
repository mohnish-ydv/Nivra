# Manual Verification After GitHub Actions Is Green

D1 is a design delivery, so the manual check confirms that the specification is
complete, readable, and internally consistent. There is no compiler to run yet.

## 1. Open the project in Termux

```bash
cd ~/storage/downloads
unzip Trion-D1-Foundation-GitHub-Ready.zip
cd Trion-D1-Foundation-GitHub-Ready
```

If the folder already exists, remove or rename the old copy before unzipping.

## 2. Run the golden verifier

```bash
bash verify.sh
```

Expected ending:

```text
Specification integrity: PASS
Design examples: PASS
Documentation anchors: PASS
★★★★★ D1 GOLDEN BUILD
```

## 3. Print the design summary

```bash
python3 tools/spec_report.py
```

Expected key values:

```text
Developer pain points: 30
P0 pain points: 12
Constitution articles: 18
Reserved keywords: 43
Design examples: 5
D1 status: PASS
```

## 4. Read the syntax tour

```bash
sed -n '1,240p' examples/design/05_complete_tour.trn
```

Check manually:

- `let` means immutable and `var` means mutable.
- braces are used for blocks.
- semicolons are not required at line endings.
- nullable values use `T?` and `none`.
- recoverable errors use `Result` and `try`.
- data records do not require constructors/getters/setters.
- inheritance is absent; behavior uses traits.
- unsafe operations are visibly contained in `unsafe {}`.

## 5. Read the decision gate

```bash
sed -n '1,260p' docs/DECISION-SUMMARY.md
```

Confirm that the exact memory model, compiler backend, public name, ABI, and
concurrency semantics are marked deferred rather than silently invented.

## Pass rule

D1 passes when GitHub Actions is green and all four local checks above match the
expected results. Then report:

```text
GG D1 Passed
```
