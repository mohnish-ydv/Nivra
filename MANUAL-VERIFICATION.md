# D4 Manual Verification After GitHub Actions Is Green

## 1. Install requirements

```bash
pkg update -y
pkg install rust python git -y
```

## 2. Run the Termux-safe cumulative verifier

From the extracted D4 folder:

```bash
bash scripts/termux-verify.sh
```

The wrapper copies the repository to:

```text
~/nivra-d4-verification
```

Expected ending:

```text
D1 regression: PASS
D2 regression: PASS
D3 regression: PASS
D4 structure: PASS
Rust tests: PASS
D4 CLI smoke tests: PASS
★★★★★ D4 GOLDEN BUILD
```

## 3. Inspect compiler status

```bash
cd ~/nivra-d4-verification
./target/debug/nivra --version
./target/debug/nivra doctor
```

Expected version:

```text
nivra 0.4.0 (parser and AST foundation D4)
```

Doctor must include:

```text
Lossless CST parser: PASS
Pratt expression parser: PASS
Typed AST foundation: PASS
Error recovery: PASS
D4 status: OPERATIONAL
```

## 4. Check the complete valid parser tour

```bash
./target/debug/nivra check examples/d4/05_complete_parser_tour.nva
```

Expected ending:

```text
0 errors
```

## 5. Confirm lossless parsing

```bash
./target/debug/nivra parse examples/d4/04_lossless_comments.nva
```

Expected:

```text
Root: source_file
Parser recoveries: 0
Errors: 0
Lossless round trip: PASS
```

## 6. Inspect the CST

```bash
./target/debug/nivra parse \
  examples/d4/02_expression_precedence.nva \
  --tree | head -n 100
```

The output must contain:

```text
function_declaration
binary_expression
if_expression
```

## 7. Confirm trivia preservation

```bash
./target/debug/nivra parse \
  examples/d4/04_lossless_comments.nva \
  --tree --trivia | grep -E 'doc_line_comment|block_comment'
```

Both token kinds must appear.

## 8. Test missing delimiter diagnostics

```bash
./target/debug/nivra check \
  examples/d4/invalid/01_missing_block_close.nva
echo $?
```

Expected:

```text
error[PAR003]
1
```

## 9. Test missing expression recovery

```bash
./target/debug/nivra check \
  examples/d4/invalid/02_missing_expression.nva
echo $?
```

Expected:

```text
error[PAR005]
1
```

## 10. Validate parser JSON

```bash
./target/debug/nivra parse \
  examples/d4/01_declarations.nva \
  --json | python3 -m json.tool >/dev/null

echo $?
```

Expected exit code:

```text
0
```

After every check passes, report:

```text
GG D4 Passed
```
