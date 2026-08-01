# Manual Verification After GitHub Actions Is Green

D3 contains real Rust code. Do not compile it directly inside Android shared
storage because executable/linker behavior there can fail. Use the included
wrapper to copy it into Termux home.

## 1. Install free Termux prerequisites

```bash
pkg update -y
pkg install rust python git -y
```

## 2. Enter the extracted D3 folder

```bash
cd ~/storage/downloads/Nivra-D3-Compiler-Foundation-GitHub-Ready
```

## 3. Run the Termux-safe verifier

```bash
bash scripts/termux-verify.sh
```

The script copies the repository to:

```text
~/nivra-d3-verification
```

and runs all cumulative checks there.

Expected ending:

```text
D1 regression: PASS
D2 regression: PASS
D3 structure: PASS
Rust tests: PASS
CLI smoke tests: PASS
★★★★★ D3 GOLDEN BUILD
```

## 4. Enter the internal test copy

```bash
cd ~/nivra-d3-verification
```

## 5. Check the CLI version and doctor

```bash
./target/debug/nivra --version
./target/debug/nivra doctor
```

Expected key lines:

```text
nivra 0.3.0 (compiler foundation D3)
Source manager: PASS
Diagnostic renderer: PASS
Lossless lexer: PASS
D3 status: OPERATIONAL
```

## 6. Check a valid file

```bash
./target/debug/nivra check examples/d3/01_hello.nva
```

Expected ending:

```text
0 errors
```

D3 does not execute the program; it checks source loading and lexing.

## 7. Check an invalid file and its exit code

```bash
./target/debug/nivra check examples/d3/invalid/unterminated_string.nva
echo $?
```

Expected:

```text
error[LEX002]: unterminated string literal
...
1
```

## 8. Inspect lossless trivia

```bash
./target/debug/nivra lex examples/d3/02_unicode_and_comments.nva --trivia | head -n 30
```

Expected token names include:

```text
whitespace
newline
doc_line_comment
block_comment
identifier
```

## 9. Check JSON output

```bash
./target/debug/nivra check examples/d3/03_literals_and_operators.nva --json | python3 -m json.tool
```

Expected JSON fields:

```text
path
tokens
errors
warnings
diagnostics
```

## Pass rule

After GitHub Actions is green and every manual check passes, report:

```text
GG D3 Passed
```
