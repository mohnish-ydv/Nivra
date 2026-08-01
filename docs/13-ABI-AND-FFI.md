# ABI and Foreign Function Interface

## V1 interoperability target

Edition 2026 supports the C ABI. Nivra does not promise direct C++ ABI compatibility.
C++ libraries require an `extern "C"` shim or generated bridge layer.

## Foreign declarations

```nva
extern "C" {
    unsafe fn strlen(value: *const U8) -> Usize
}
```

Calling a foreign function requires `unsafe(ffi)` unless the declaration is wrapped
by a verified safe Nivra API.

## Layout

- `@repr(C)` requests C-compatible field order and alignment for eligible structs.
- Enums do not receive C layout unless an explicit representation is selected.
- `Bool`, `Char`, `String`, slices, traits, and generic types have no implicit C ABI.
- Fixed-width integers should be used at boundaries.
- Foreign ownership and deallocation responsibility must be documented in types or
  wrapper APIs.

## Exporting Nivra

A `pub extern "C" fn` may export a C-callable symbol when all parameter and return
types are ABI-safe. Panics cannot cross the boundary because panic aborts. Result
values require an explicit C representation or wrapper convention.

## Strings and slices

FFI uses explicit pointer-length pairs or declared foreign string conventions.
Nivra `String` is not passed directly. Safe wrappers validate encoding and ownership.

## Bindings

A future `nivra bindgen` command may generate declarations from C headers. Generated
bindings remain unsafe until wrapped. Arbitrary C preprocessor behavior is not
reimplemented in the Nivra language.

## Auditability

Every foreign declaration, foreign call site, raw pointer conversion, and manual
sendability assertion appears in the unsafe-capability report.
