# D6 Build-Fix Termux and GitHub Commands

## 1. Extract the corrected archive

```bash
cd ~/storage/downloads
rm -rf Nivra-D6-Type-Checker-Build-Fix-GitHub-Ready
unzip Nivra-D6-Type-Checker-Build-Fix-GitHub-Ready.zip
```

## 2. Preserve the Git history from the failed D6 folder

```bash
mv Nivra-D6-Type-Checker-GitHub-Ready/.git \
   Nivra-D6-Type-Checker-Build-Fix-GitHub-Ready/.git
```

## 3. Commit and push the correction

```bash
cd ~/storage/downloads/Nivra-D6-Type-Checker-Build-Fix-GitHub-Ready
git config --global --add safe.directory "$PWD"
git add -A
git commit -m "fix: repair D6 Cargo test dependency graph"
git push
```

The corrected workflow first checks the local Cargo dependency graph and lockfile,
then runs all Rust tests, cumulative verification, release build, and CLI smoke.

## 4. Verify after Actions turns green

```bash
pkg update -y
pkg install rust python git unzip -y
cd ~/storage/downloads/Nivra-D6-Type-Checker-Build-Fix-GitHub-Ready
bash scripts/termux-verify.sh
```

Then follow `MANUAL-VERIFICATION.md`.
