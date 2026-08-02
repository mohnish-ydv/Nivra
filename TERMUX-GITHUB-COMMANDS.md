# D7 Exact Termux and GitHub Commands

## Extract

```bash
cd ~/storage/downloads
rm -rf Nivra-D7-Nominal-Members-GitHub-Ready
unzip Nivra-D7-Nominal-Members-GitHub-Ready.zip
```

## Preserve D6 Git history

```bash
mv Nivra-D6-Type-Checker-Final-Build-Fix-GitHub-Ready/.git \
   Nivra-D7-Nominal-Members-GitHub-Ready/.git
```

## Commit and push

```bash
cd ~/storage/downloads/Nivra-D7-Nominal-Members-GitHub-Ready

git config --global --add safe.directory "$PWD"

git add -A
git commit -m "feat: implement Nivra D7 nominal types and members"
git push
```

Open:

```text
GitHub → Nivra → Actions → Verify D7 Nominal Members
```

## Green Action ke baad

```bash
pkg update -y
pkg install rust python git unzip -y

cd ~/storage/downloads/Nivra-D7-Nominal-Members-GitHub-Ready
bash scripts/termux-verify.sh
```

Then follow `MANUAL-VERIFICATION.md`.
