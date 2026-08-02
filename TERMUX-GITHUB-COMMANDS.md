# D8 Termux and GitHub Commands

```bash
cd ~/storage/downloads
rm -rf Nivra-D8-Generics-Traits-GitHub-Ready
unzip Nivra-D8-Generics-Traits-GitHub-Ready.zip
```

Transfer the verified D7 Git history:

```bash
mv Nivra-D7-Nominal-Members-Verified-Final-GitHub-Ready/.git \
   Nivra-D8-Generics-Traits-GitHub-Ready/.git
```

Commit and push:

```bash
cd ~/storage/downloads/Nivra-D8-Generics-Traits-GitHub-Ready
git config --global --add safe.directory "$PWD"
git add -A
git commit -m "feat: implement Nivra D8 generics and trait constraints"
git push
```

Open `Actions → Verify D8 Generics and Traits`.

After the workflow is green:

```bash
pkg update -y
pkg install rust python git unzip -y
cd ~/storage/downloads/Nivra-D8-Generics-Traits-GitHub-Ready
bash scripts/termux-verify.sh
```
