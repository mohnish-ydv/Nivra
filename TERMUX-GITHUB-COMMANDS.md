# Exact Termux and GitHub Commands

Replace the repository URL with your own GitHub repository.

## First-time phone setup

```bash
pkg update -y
pkg install git python unzip -y
termux-setup-storage
```

## Verify downloaded ZIP

```bash
cd ~/storage/downloads
unzip Trion-D1-Foundation-GitHub-Ready.zip
cd Trion-D1-Foundation-GitHub-Ready
bash verify.sh
```

## Push to a new GitHub repository

Create an empty repository on GitHub without README, license, or `.gitignore`,
then run:

```bash
cd ~/storage/downloads/Trion-D1-Foundation-GitHub-Ready

git config --global --add safe.directory "$PWD"
git init
git branch -M main
git add .
git commit -m "feat: complete Trion D1 foundation"
git remote add origin https://github.com/YOUR_USERNAME/YOUR_REPOSITORY.git
git push -u origin main
```

If Git asks for authentication, use your GitHub username and a Personal Access
Token instead of your account password.

## Later fixes

```bash
git add .
git commit -m "fix: stabilize D1 verification"
git push
```

After pushing, open the repository's **Actions** tab and check that
`Verify D1 Foundation` has a green tick. Then follow `MANUAL-VERIFICATION.md`.
