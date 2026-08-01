# Memory Model — Edition 2026

## Goal

Nivra aims to prevent use-after-free, double-free, invalid aliasing, and data
races in safe code without requiring explicit lifetime parameters in ordinary
source.

## Ownership model

Every value has an owner unless it is a non-owning borrow. Non-`Copy` values move
when transferred. Ownership transfer is visible through use-after-move checks,
not through mandatory syntax at each call site.

```nva
let request = Request.new()
send(request)       // ownership moves
// request is unavailable here
```

## Storage forms

### Inline and aggregate values

Records, structs, enums, tuples, and arrays use value semantics. The compiler may
optimize representation without changing move, copy, drop, or aliasing behavior.

### `Box<T>`

`Box<T>` is unique heap ownership. Moving the box transfers ownership. Borrowing
its value does not transfer ownership.

### `Shared<T>` and `Weak<T>`

`Shared<T>` provides explicit thread-safe atomic reference counting. `Weak<T>`
observes shared data without extending its lifetime and is required to break
ownership cycles.

The core language does not silently convert owned values to shared values.

## Borrows

- `&T` is a shared read-only borrow.
- `&mut T` is an exclusive mutable borrow.
- Any number of shared borrows or one mutable borrow may exist for the same value,
  but never both at the same time.
- A borrow cannot outlive its owner.
- A borrow cannot cross an `await` suspension point.
- User-defined records and structs cannot store borrowed fields in Edition 2026.
- Functions may return a borrow only when its origin is unambiguous from exactly
  one borrowed input. Otherwise an owned return is required.
- Edition 2026 has no user-written lifetime parameters.

These restrictions intentionally trade some advanced zero-copy patterns for a
simpler, teachable safety model.

## Destruction

- Owned values are destroyed deterministically.
- Scope locals drop in reverse declaration order.
- `Shared<T>` destroys its payload when the final strong reference is released.
- Types may implement `Drop` for cleanup.
- `Drop` cannot fail and cannot be called directly.
- Values are considered moved before their destructor could run, preventing
  double destruction.

## `defer`

`defer` schedules an action at normal scope exit. Deferred actions run last-in,
first-out while scope locals remain alive, followed by automatic local drops.
Because Edition 2026 panic terminates the process, deferred actions are not
promised after panic.

## Allocation

- Stack versus heap placement is an implementation decision except where an API
  explicitly requires stable address or foreign layout.
- `Box`, `Shared`, and collections may allocate.
- Allocation failure is represented by a runtime abort in the default global
  allocator model; fallible allocator APIs are available for systems code.
- Custom allocators are explicit generic or context parameters, not ambient global
  mutation.

## Raw memory

Raw pointers are `*const T` and `*mut T`. Dereference, pointer arithmetic,
manual allocation, manual deallocation, and foreign pointer conversion require a
named unsafe capability.

```nva
unsafe(memory) {
    let pointer: *mut U8 = memory.allocate(bytes: 128)
    pointer.write(0, 42)
    memory.release(pointer)
}
```

## Named unsafe capabilities

Unsafe blocks declare one or more reasons:

- `memory` — raw pointers, layout, manual allocation
- `ffi` — foreign calls and foreign-owned values
- `concurrency` — low-level synchronization contracts
- `platform` — inline assembly, syscalls, target intrinsics

```nva
unsafe(memory, ffi) {
    native_copy(destination, source, length)
}
```

The compiler records capability sites for audit tooling.

## Concurrency relationship

`Send` and `Sync` are compiler-known auto traits:

- `Send` means ownership may cross task/thread boundaries.
- `Sync` means shared references are safe across boundaries.
- Raw pointers are neither by default.
- `Shared<T>` crossing boundaries requires appropriate payload capabilities.

## Garbage collection policy

Nivra has no mandatory tracing garbage collector. A library or application may
host an arena, tracing heap, ECS storage, or domain-specific collector explicitly.
Such memory does not weaken safe-code rules at its API boundary.
