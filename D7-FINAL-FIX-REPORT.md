# D7 Final Build Fix Report

## GitHub Actions evidence

The uploaded no-fail-fast Rust test log proved that every D7 workspace target
compiled successfully on pinned Rust 1.74.0. Two independent tests failed:

1. `explain_supports_nominal_diagnostics` expected the public NOM001 explanation
   to contain the word `member`, while the explanation only listed field, method,
   and enum variant.
2. `rejects_record_syntax_for_enum` did not receive NOM010 because the parser's
   empty-record heuristic inspected lossless text with leading trivia. `State { }`
   could therefore be split into a name plus a block instead of a record expression.

## Corrections

- NOM001 now explicitly says `member (field, method, or enum variant)`.
- Empty record construction trims leading/trailing trivia before the uppercase
  nominal-name check.
- Added a lossless parser regression for `Empty { }`.
- Added direct CLI conformance proving enum record syntax emits NOM010.
- Added focused root-cause tests before the complete workspace suite.
- Added formatting verification before compilation.
- The structure gate now rejects removal of any of these fixes.

## Evidence from the failed run

Before these corrections, `cargo check --workspace --all-targets --locked` passed
for all eight crates. The complete no-fail-fast suite exposed exactly the two
failures documented above; all remaining tests passed.

## Release gate

This archive is a final corrected release candidate. Authoritative acceptance still
requires the included GitHub Actions workflow to turn green and the Termux verifier
to print `★★★★★ D7 GOLDEN BUILD`.
