# Identity and Governance

## Selected pre-1.0 identity

- Name: **Nivra**
- Pronunciation: `NIV-rah`
- CLI: `nivra`
- Extension: `.nva`
- Manifest: `nivra.toml`
- Lockfile: `nivra.lock`
- Edition: `2026`
- Tagline: **Power without the pain.**
- License: Apache-2.0

## Why Trion was retired

The D1 name was explicitly provisional. A search performed on 2026-08-02 found
an older Trion operating-system project, a TRION hardware SDK, FPGA products, and
other established technology uses. Keeping it would create avoidable search and
identity confusion.

Reference pages reviewed:

- `https://trion.sourceforge.net/`
- `https://dewetron.github.io/TRION-SDK/`
- `https://www.efinixinc.com/products/trion/`

## Nivra review scope

Searches for the exact phrases “Nivra programming language” and “Nivra compiler”
did not surface an established language/compiler in the reviewed results. The
name does have unrelated uses, including an Android application and a nonprofit.
That means the name is suitable for pre-1.0 engineering continuity, but this
review is not legal, trademark, registry, domain, or package-namespace clearance.

Known unrelated references reviewed:

- `https://play.google.com/store/apps/details?id=com.northbyte.nivra`
- `https://nivra.org/`

## Naming policy

- The repository and specification use Nivra from D2 onward.
- A rename before 1.0 requires an RFC, migration map, collision evidence, and
  automated identity replacement checks.
- After 1.0, renaming is considered a compatibility-breaking governance action.
- Package scopes and official registry ownership are release gates before public
  registry launch.

## Governance

Language changes use an RFC process. An accepted RFC must include:

1. problem and affected pain-map IDs
2. proposed semantics and syntax
3. rejected alternatives
4. complexity and implementation cost
5. compatibility and migration impact
6. diagnostic behavior
7. security and performance implications
8. normative specification patch
9. conformance tests

The constitution has higher authority than convenience features. A feature that
violates the constitution requires a constitutional amendment first.
