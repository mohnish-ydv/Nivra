# D5 Termux and GitHub Commands

## Extract

```bash
cd ~/storage/downloads
rm -rf Nivra-D5-Semantic-Resolution-GitHub-Ready
unzip Nivra-D5-Semantic-Resolution-GitHub-Ready.zip
```

## Preserve D4 Git history

```bash
mv Nivra-D4-Parser-AST-GitHub-Ready/.git \
   Nivra-D5-Semantic-Resolution-GitHub-Ready/.git
```

## Commit and push

```bash
cd ~/storage/downloads/Nivra-D5-Semantic-Resolution-GitHub-Ready
git config --global --add safe.directory "$PWD"
git add -A
git commit -m "feat: implement Nivra D5 semantic name resolution"
git push
```

Then open GitHub **Actions** and verify `Verify D5 Semantic Resolution` is green.
After that, follow `MANUAL-VERIFICATION.md`.
