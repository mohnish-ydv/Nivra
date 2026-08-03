# D9 Termux and GitHub Commands

## Extract

```bash
cd ~/storage/downloads
rm -rf Nivra-D9-Ownership-Borrow-Foundation-GitHub-Ready
unzip Nivra-D9-Ownership-Borrow-Foundation-Final-Build-Fix-GitHub-Ready.zip
```

## Preserve existing repository history

Use the existing cloned Nivra repository as the destination; do not move a stale `.git` directory into an unrelated folder.

```bash
cd ~/storage/downloads
rm -rf Nivra-repo
git clone https://github.com/mohnish-ydv/Nivra.git Nivra-repo
cp -R Nivra-D9-Ownership-Borrow-Foundation-GitHub-Ready/. Nivra-repo/
cd Nivra-repo
git config --global --add safe.directory "$PWD"
```

## Review, commit, and push

```bash
git status --short
git add -A
git commit -m "feat: add D9 ownership and borrow checker foundation"
git push origin main
```

Then open `Actions → Verify D9 Ownership and Borrow Foundation`. Do not treat D9 as verified until the workflow is green.

## Termux executable verification

```bash
pkg update -y
pkg install rust python git unzip -y
cd ~/storage/downloads/Nivra-D9-Ownership-Borrow-Foundation-GitHub-Ready
bash scripts/termux-verify.sh
```

Expected final marker:

```text
★★★★★ D9 GOLDEN BUILD
```

## Direct recovery commands after a failure

```bash
cd ~/nivra-d9-verification
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked --no-fail-fast
```

Fix every reported root cause, rerun all three commands, then rerun `bash verify.sh`; do not stop after the first failure.
