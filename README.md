# Trion — D1 Foundation

> Working name: **Trion**  
> Mission: **Power without the pain.**

This repository is the first design delivery for a new statically typed, compiled,
general-purpose programming language focused on removing recurring developer
headaches: build complexity, dependency conflicts, hostile diagnostics, null and
memory bugs, fragmented tooling, unsafe concurrency, and unnecessary boilerplate.

## D1 scope

D1 completes three internal checkpoints:

- **M0 — Mission and Developer Pain Map**
- **M1 — Language Constitution**
- **M2 — Syntax Direction v0.1**

D1 intentionally contains no compiler. It creates the rules that the compiler,
standard library, package manager, formatter, linter, and IDE tooling must follow.

## Verify on Termux or GitHub Actions

```bash
bash verify.sh
```

Expected final line:

```text
★★★★★ D1 GOLDEN BUILD
```

## Repository map

- `docs/` — human-readable design documents
- `spec/d1/` — machine-readable D1 decisions
- `examples/design/` — non-executable syntax design samples
- `tools/` — specification validation and reporting
- `.github/workflows/` — zero-cost CI verification
- `DELIVERY-REPORT.md` — delivery status and evidence
- `ACCEPTANCE-CHECKLIST.md` — pass/fail criteria
- `MANUAL-VERIFICATION.md` — exact checks after GitHub Actions turns green

## Important status

The name **Trion**, compiler command `trion`, and extension `.trn` are working
identifiers. Public naming and collision checks are deliberately deferred until
the architecture decisions are locked. All other locked D1 decisions are listed
in `docs/DECISION-SUMMARY.md`.
