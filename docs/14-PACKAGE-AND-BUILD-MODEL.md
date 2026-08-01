# Package and Build Model

## Project layout

```text
project/
├── nivra.toml
├── nivra.lock
├── src/
│   ├── main.nva
│   └── lib.nva
├── tests/
├── examples/
└── assets/
```

## Manifest

`nivra.toml` is the single authored project configuration for ordinary projects.
It contains package identity, edition, targets, dependencies, features, native
requirements, and profile overrides.

```toml
[package]
name = "weather-cli"
version = "0.1.0"
edition = "2026"

[dependencies]
http = "1.2"
json = "2.0"
```

## Lockfile

Applications and executable workspaces commit `nivra.lock`. Libraries publish
compatible dependency ranges but CI also tests a lockfile. Resolution records
exact versions, content hashes, registry source, enabled features, and target
selection.

## Security

- Packages cannot run arbitrary install scripts by default.
- Package content is hash-verified.
- Registry metadata is signed or delivered through an authenticated integrity
  mechanism.
- Native libraries are declared capabilities with visible platform requirements.
- Network access can be disabled after all locked artifacts are cached.
- The toolchain produces a dependency and license report.

## Workspaces

One workspace may contain multiple packages with a shared lockfile and target
cache. Cyclic package dependencies are rejected.

## Features

Features are additive. A feature must not silently remove API or safety behavior.
Mutually exclusive platform choices use target conditions, not negative features.

## CLI

The integrated command surface includes:

```text
nivra new
nivra init
nivra check
nivra run
nivra build
nivra test
nivra fmt
nivra lint
nivra fix
nivra doc
nivra add
nivra remove
nivra update
nivra tree
nivra audit
nivra clean
nivra doctor
```

## Build scripts

Edition 2026 has no arbitrary dependency install script. Projects needing code
generation use declared, sandboxed tools with explicit inputs, outputs, permissions,
and reproducibility metadata. The first compiler implementation may postpone this
facility while preserving the rule.
