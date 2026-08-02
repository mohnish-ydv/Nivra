# D7 Delivery Report

## Delivery identity

- Delivery: D7
- Version: 0.7.0
- Title: Nominal Types and Member Checking
- Builds on: verified D1–D6
- Workspace crates: 8
- Third-party Rust dependencies: 0
- Minimum Rust: 1.74
- Status before user verification: VERIFIED-FORMAT RELEASE CANDIDATE

## GitHub Actions build correction

The first D7 CI run exposed one Rust source-syntax defect in a unit test:
an embedded Nivra string used unescaped quotes around `done`. The corrected
fixture uses `State.redy(\"done\")`; `redy` remains intentionally misspelled
to test NOM001 suggestions. An early Rust-string suffix preflight now prevents
this class of defect from reaching Cargo compilation again.

## Final GitHub Actions corrections

The no-fail-fast run compiled every target and exposed two remaining test failures.
The final revision aligns the NOM001 explanation with its CLI contract and fixes
the parser's empty-constructor leading-trivia ambiguity so enum record syntax
reliably reaches NOM010 validation. Dedicated parser, type-checker, and CLI tests
now guard both cases before the full suite.


## Final formatting correction

The latest uploaded run passed Rust toolchain installation, Cargo dependency
validation, and D7 structural preflight, then stopped exclusively at
`cargo fmt --all -- --check`. The complete Rust 1.74 rustfmt output from that run
contained 159 hunks across eight Rust files. Every reported hunk is applied in
this revision. Termux verification no longer auto-formats source, so formatting
drift cannot be hidden locally.

## Implemented

1. Lossless record-construction parsing.
2. Record, struct, and enum body indexing.
3. Field types, defaults, and visibility metadata.
4. Unit and tuple-payload enum variant typing.
5. Inherent and trait implementation method collection.
6. `Self` substitution in method signatures.
7. Field and method lookup on nominal values.
8. Mutable receiver and member-assignment validation.
9. Constructor missing/unknown/duplicate/type checks.
10. NOM001–NOM010 diagnostics.
11. `nivra typecheck --nominals`.
12. Nominal data in JSON output.
13. Cumulative D1–D7 CI and Termux verification.

## Regression protections

D7 retains permanent guards for both D6 failures:

- `nivra-types` declares its parser test dependency.
- reserved keyword `ok` is never used as a normal binding in the primitive test.

D7 also adds a parser ambiguity test proving that `if enabled { ... }` is not
misclassified as a record constructor.

## Deliberate limitations

- Generic nominal constructors use recovery types; complete substitution is D8.
- Cross-module visibility checks are deferred.
- Trait selection and overlapping implementation analysis are deferred.
- Record-payload enum variants are not field-typed yet.
- No ownership-flow checker or code generation is claimed.

## Acceptance

The delivery passes only after GitHub Actions is green and the manual Termux
verification ends with `★★★★★ D7 GOLDEN BUILD`.
