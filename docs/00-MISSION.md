# Mission

## Product statement

Trion is a statically typed, compiled, general-purpose programming language and
official toolchain designed to remove recurring developer headaches without
sacrificing native performance or low-level capability.

Its purpose is not to imitate C++. Its purpose is to provide comparable power
while rejecting accidental complexity that developers should not have to carry.

## Core promise

> A developer should spend most of their time expressing intent and solving the
> product problem—not configuring tools, decoding compiler output, repairing
> dependency graphs, or defending against avoidable language hazards.

## Target users

### Primary

- application and systems developers who need native performance
- small teams that cannot maintain a complicated build ecosystem
- developers moving between CLI, backend, desktop, Android, and WebAssembly work
- learners who need a serious language without hostile defaults
- experienced C/C++ developers who want safer defaults and unified tooling

### Secondary

- library authors
- game and engine developers
- automation developers
- embedded developers after the core language stabilizes
- organizations that value deterministic builds and long-term maintenance

## V1 target workload

V1 prioritizes:

1. native command-line tools
2. automation and developer utilities
3. backend services
4. reusable native libraries
5. portable application logic

Android UI frameworks, game engines, kernels, embedded boards, GPU languages,
and browser UI frameworks are ecosystem goals after the language core is stable.

## Success metrics

The project will measure success using concrete developer outcomes:

- a new project can be created, built, tested, formatted, linted, documented, and
  packaged through one official command-line tool
- clean projects build without manually authored build scripts
- dependency resolution produces a reproducible lockfile
- every diagnostic contains location, cause, and at least one actionable next step
  when a safe suggestion exists
- null dereference is impossible in safe code
- use-after-free and double-free are impossible in safe code
- unhandled recoverable errors are compiler-visible
- structured concurrency prevents orphaned tasks by default
- equivalent common tasks require materially less ceremony than C++
- a project can be cloned and verified on Android + Termux without paid services

## Anti-goals

Trion will not optimize for:

- source compatibility with C or C++
- supporting every programming paradigm equally
- clever syntax at the cost of readability
- hidden runtime behavior
- multiple competing official build systems
- unchecked exceptions as routine control flow
- ambient nullability
- unrestricted compile-time metaprogramming in V1
- immediate support for every operating system and architecture
- benchmark wins obtained through unsafe defaults

## Decision test

Every proposed language feature must answer:

1. Which recurring developer problem does it solve?
2. Is the problem frequent or severe enough to belong in the language?
3. Can tooling or a library solve it with less permanent complexity?
4. What new failure modes does the feature introduce?
5. Can a new developer understand its behavior from local code?
6. Can the compiler provide precise diagnostics for misuse?

A feature without a strong answer is rejected or postponed.
