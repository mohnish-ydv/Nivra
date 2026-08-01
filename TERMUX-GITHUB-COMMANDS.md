# D4 Termux and GitHub Commands

## Extract and preflight

```bash
cd ~/storage/downloads
rm -rf Nivra-D4-Parser-AST-GitHub-Ready
unzip Nivra-D4-Parser-AST-GitHub-Ready.zip
cd Nivra-D4-Parser-AST-GitHub-Ready
python3 tools/d4_structure_lint.py
```

## Preserve the existing D3 Git history

Run from `~/storage/downloads`:

```bash
mv Nivra-D3-Compiler-Foundation-GitHub-Ready/.git \
   Nivra-D4-Parser-AST-GitHub-Ready/.git
```

Then push:

```bash
cd ~/storage/downloads/Nivra-D4-Parser-AST-GitHub-Ready

git config --global --add safe.directory "$PWD"
git add -A
git commit -m "feat: implement Nivra D4 parser and AST foundation"
git push
```

Open:

```text
GitHub repository → Actions → Verify D4 Parser and AST
```

After the green check, follow `MANUAL-VERIFICATION.md`.
