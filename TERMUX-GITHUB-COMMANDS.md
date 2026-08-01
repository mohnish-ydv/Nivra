# Exact Termux and GitHub Commands

## Install requirements

```bash
pkg update -y
pkg install git python unzip -y
termux-setup-storage
```

## Verify the D2 ZIP

```bash
cd ~/storage/downloads
unzip Nivra-D2-Architecture-Spec-GitHub-Ready.zip
cd Nivra-D2-Architecture-Spec-GitHub-Ready
bash verify.sh
```

## Replace the existing repository with cumulative D2

Run inside the extracted D2 folder. Replace the repository URL only when your
actual repository uses a different name.

```bash
cd ~/storage/downloads/Nivra-D2-Architecture-Spec-GitHub-Ready

git config --global --add safe.directory "$PWD"
git init
git branch -M main
git add .
git commit -m "feat: complete Nivra D2 architecture specification"
git remote add origin https://github.com/mohnish-ydv/Nivra.git
git push -u origin main
```

If `origin` already exists:

```bash
git remote set-url origin https://github.com/mohnish-ydv/Nivra.git
git push -u origin main
```

If you are updating the same local Git repository instead of starting from this
ZIP, copy the D2 contents into it, then commit and push normally.

After pushing, open **Actions** and verify `Verify Nivra D2` is green. Then run
the steps in `MANUAL-VERIFICATION.md`.
