# D6 Termux and GitHub Commands

## Extract

```bash
cd ~/storage/downloads
rm -rf Nivra-D6-Type-Checker-GitHub-Ready
unzip Nivra-D6-Type-Checker-GitHub-Ready.zip
```

## Preserve D5 Git history

```bash
mv Nivra-D5-Semantic-Resolution-GitHub-Ready/.git \
   Nivra-D6-Type-Checker-GitHub-Ready/.git
```

## Commit and push

```bash
cd ~/storage/downloads/Nivra-D6-Type-Checker-GitHub-Ready
git config --global --add safe.directory "$PWD"
git add -A
git commit -m "feat: implement Nivra D6 static type checker"
git push
```

## Verify after Actions turns green

```bash
pkg update -y
pkg install rust python git -y
bash scripts/termux-verify.sh
```

Then follow `MANUAL-VERIFICATION.md`.
