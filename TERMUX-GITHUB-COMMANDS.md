# D8 Final Build-Fix Termux and GitHub Commands

## Extract

```bash
cd ~/storage/downloads
rm -rf Nivra-D8-Generics-Traits-Final-Build-Fix-GitHub-Ready
unzip Nivra-D8-Generics-Traits-Final-Build-Fix-GitHub-Ready.zip
```

## Transfer the failed D8 repository history

```bash
mv Nivra-D8-Generics-Traits-GitHub-Ready/.git \
   Nivra-D8-Generics-Traits-Final-Build-Fix-GitHub-Ready/.git
```

## Commit and push

```bash
cd ~/storage/downloads/Nivra-D8-Generics-Traits-Final-Build-Fix-GitHub-Ready
git config --global --add safe.directory "$PWD"
git add -A
git commit -m "fix: finalize D8 generic and enum diagnostic pipelines"
git push
```

Open `Actions → Verify D8 Generics and Traits`.

## After GitHub Actions is green

```bash
pkg update -y
pkg install rust python git unzip -y
cd ~/storage/downloads/Nivra-D8-Generics-Traits-Final-Build-Fix-GitHub-Ready
bash scripts/termux-verify.sh
```

The Cargo verification copy is created at `~/nivra-d8-verification`.
