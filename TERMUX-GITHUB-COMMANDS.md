# Termux and GitHub Commands for D3

## Verify before pushing

```bash
pkg update -y
pkg install rust python git -y

cd ~/storage/downloads/Nivra-D3-Compiler-Foundation-GitHub-Ready
bash scripts/termux-verify.sh
```

## Preserve the existing Nivra repository history

Assuming the verified D2 folder still contains `.git`:

```bash
cd ~/storage/downloads

mv Nivra-D2-Architecture-Spec-GitHub-Ready/.git \
Nivra-D3-Compiler-Foundation-GitHub-Ready/.git
```

If D2 was already pushed and the old folder no longer has `.git`, clone the
repository into Termux home and copy D3 over it instead.

## Commit and push

```bash
cd ~/storage/downloads/Nivra-D3-Compiler-Foundation-GitHub-Ready

git config --global --add safe.directory "$PWD"
git add -A
git commit -m "feat: implement Nivra D3 compiler foundation"
git push
```

## GitHub Actions result

Open **Actions → Verify D3 Compiler Foundation**.

A green run confirms:

- D1 and D2 regressions
- D3 structure validation
- Rust formatting
- all workspace tests
- CLI smoke tests
- release build
- uploaded Linux x86_64 `nivra` artifact

The uploaded Linux artifact is not an Android binary. On Termux, compile natively
with `bash scripts/termux-verify.sh`.
