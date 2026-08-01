# Backend and Portability Strategy

## Reference backend: C11

The first native backend lowers Nivra MIR to a controlled portable C11 subset and
uses Clang to produce object files and executables.

Why this is the first backend:

- Clang is available in Termux and common CI environments.
- C11 offers broad platform reach and straightforward C ABI integration.
- Generated C is inspectable during bootstrap debugging.
- The frontend and language semantics can mature before direct machine-code work.

Generated C is an implementation artifact, not a stable human-facing source API.
Nivra semantics must not depend on C undefined behavior. The backend emits checks,
well-defined unsigned operations, explicit layouts, and runtime helpers as needed.

## Optimized backend: LLVM

An LLVM backend is planned after MIR and conformance tests stabilize. It consumes
the same backend-neutral MIR and provides richer optimization, debug metadata, and
architecture support. LLVM does not replace the reference C backend immediately;
both are checked against the same conformance suite.

## Target sequence

1. Linux / Android-Termux AArch64 development host
2. Linux x86-64 CI and release host
3. Android NDK application/library target
4. Windows x86-64
5. macOS Apple Silicon and x86-64 where CI access permits
6. WebAssembly WASI
7. embedded targets after allocator and runtime profiles exist

## Profiles

- `debug`: full checks, debug information, minimal optimization
- `release`: full language safety checks, optimized
- `size`: full language safety checks, size-oriented optimization
- `unsafe-fast`: not a standard profile; checks may only be disabled through
  explicit operations in source or narrowly documented target configuration

Safety behavior does not silently change between debug and release.

## Runtime split

The runtime is modular:

- core: panic, allocation hooks, checked operations, type metadata needed by ABI
- std: files, networking, collections, formatting, processes
- async: executor, timers, I/O reactor, cancellation
- test: test runner and deterministic runtime hooks

Freestanding and embedded profiles can exclude higher layers.

## Reproducibility

Build fingerprints include compiler version, edition, target, profile, enabled
features, manifest, lockfile, environment capabilities, and source hashes. Absolute
host paths and timestamps are removed from deterministic artifacts where the
platform permits.
