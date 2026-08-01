# D2 Acceptance Checklist

## Automated

- [x] D1 verification still passes unchanged.
- [x] Every D2 JSON document is valid.
- [x] Decision IDs and rule IDs are unique.
- [x] Locked, deferred, and rejected sets do not overlap.
- [x] Identity files consistently use Nivra, `nivra`, and `.nva`.
- [x] The EBNF has unique productions and no undefined nonterminal references.
- [x] Required type, memory, error, concurrency, compiler, FFI, package, and
      compatibility rules are present.
- [x] D2 examples use balanced delimiters and current identity.
- [x] The complete tour covers safety, errors, traits, generics, structured
      concurrency, FFI, and explicit unsafe capabilities.
- [x] Semantic contradiction checks pass.
- [x] GitHub Actions invokes the same cumulative verifier used in Termux.

## Manual

- [ ] GitHub Actions shows a green check for `Verify Nivra D2`.
- [ ] `bash verify.sh` ends with `★★★★★ D2 GOLDEN BUILD`.
- [ ] `python3 tools/d2_report.py` prints the expected architecture counts.
- [ ] `python3 tools/decision_query.py memory` prints the locked memory summary.
- [ ] The complete D2 syntax tour is readable and internally coherent.
- [ ] The user accepts the D2 architecture and reports `GG D2 Passed`.

## Gate

D3 starts only after all manual checks pass.
