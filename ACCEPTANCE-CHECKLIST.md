# D1 Acceptance Checklist

D1 passes only when every required item below is satisfied.

## Automated

- [x] `bash verify.sh` exits with code 0.
- [x] All machine-readable JSON files parse successfully.
- [x] Pain IDs, constitution IDs, and keywords are unique.
- [x] At least 25 real developer pain points are documented.
- [x] All P0 pain categories required by the mission are present.
- [x] At least 15 constitutional rules are locked.
- [x] Syntax examples contain balanced delimiters.
- [x] No unresolved drafting markers remain.
- [x] GitHub Actions invokes the same verifier used locally.

## Manual

- [ ] GitHub Actions shows a green tick.
- [ ] `bash verify.sh` prints `★★★★★ D1 GOLDEN BUILD`.
- [ ] `python3 tools/spec_report.py` reports 30 pain points and 18 articles.
- [ ] The syntax tour is readable on a phone and contains no unexplained construct.
- [ ] Locked, deferred, and rejected decisions are clearly separated.
- [ ] The developer agrees that D1 reflects the project mission.

## Delivery gate

The next delivery should start only after the user reports:

```text
GG D1 Passed
```
