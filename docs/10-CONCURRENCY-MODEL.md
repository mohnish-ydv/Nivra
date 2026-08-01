# Concurrency Model — Edition 2026

## Principles

- concurrency is structured
- ownership crossing is checked
- cancellation is cooperative and propagated
- scheduling is not observable language semantics
- shared mutation is explicit
- blocking work is distinguished from asynchronous work

## Async functions

`async fn` returns a lazy `Task<T, E>`-compatible computation. It begins when
spawned or awaited according to the runtime API. `await` is only legal in async
contexts.

## Task groups

Ordinary task creation occurs through a lexical task group:

```nva
async fn load_all(paths: List<Path>) -> Result<List<Data>, IoError> {
    task_group group {
        let tasks = paths.map(|path| group.spawn async {
            try await load(path)
        })
        try await Task.all(tasks)
    }
}
```

A task group cannot exit while children remain active. On early error or
cancellation it requests cancellation of remaining children, waits for them to
finish, and then returns.

## Cancellation

- Cancellation is cooperative.
- Await points check cancellation unless an API documents otherwise.
- Long CPU work calls `cancel.check()` periodically.
- Cancellation is represented separately from domain errors at the runtime layer.
- Cleanup uses normal `Result` paths and deterministic destruction.

## Ownership crossing

A spawned closure must own or safely share every captured value. Non-`Send` values
cannot cross execution boundaries. Borrows cannot cross an `await` or outlive a
task-group scope.

## Shared state

Safe shared mutation uses explicit standard primitives:

- `Mutex<T>`
- `RwLock<T>`
- `Atomic<T>` for supported scalar types
- typed `Channel<T>`
- actor-style owned task loops

Lock poisoning is not a language feature because panic aborts the process.

## Sendability

- `Send` and `Sync` are auto traits checked by the compiler.
- Types may opt out through representation or unsafe internals.
- Manually asserting either trait requires `unsafe(concurrency)` and a documented
  invariant.

## Detached work

Detached tasks are not created through bare `spawn`. `runtime.detach(task)` is an
explicit advanced operation returning a `DetachedHandle`. Dropping the handle
without an explicit policy is a warning in standard projects.

## Blocking work

Blocking system or foreign calls must be declared `blocking` by their API. Async
code invokes them through `task.blocking`, allowing the runtime to isolate worker
capacity.

## Runtime

The standard runtime provides:

- a work-stealing executor on supported native targets
- timers and cancellation
- asynchronous I/O adapters
- a bounded blocking pool
- deterministic test mode with virtual time hooks

Library code depends on runtime interfaces rather than mutating an invisible
global executor.

## Ordering and determinism

Task scheduling order is unspecified. Programs requiring order must express it
through awaits, channels, locks, or explicit sequencing. Data-race freedom does
not imply deterministic output.

## Exclusions

Edition 2026 does not provide implicit detached tasks, green-thread identity as a
stable API, automatic shared-memory synchronization, or borrow-across-await.
