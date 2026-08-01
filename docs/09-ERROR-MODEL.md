# Error Model — Edition 2026

## Two failure classes

Nivra separates recoverable operational failure from unrecoverable program defects.

### Recoverable failure

Expected failures use `Result<T, E>`:

- file not found
- malformed input
- network timeout
- permission denied
- validation failure
- unavailable external service

### Program defect

Broken invariants use `panic`, assertions, or runtime traps:

- out-of-bounds access after a proven logic defect
- integer overflow under checked arithmetic
- impossible internal state
- failed compiler-generated safety check

## Result propagation

`try expression` unwraps `ok(value)` or returns a compatible `err(error)` from
the current function.

```nva
fn load(path: Path) -> Result<Config, ConfigError> {
    let text = try fs.read_text(path)
    let value = try json.parse(text)
    Config.decode(value)
}
```

Error conversion during propagation requires an explicit `From<SourceError>`
implementation for the destination error type. The diagnostic identifies the
missing conversion.

## Validation sugar

`ensure condition else error` returns `err(error)` from a `Result`-returning
function when the condition is false.

```nva
ensure port <= 65535 else ConfigError.invalid_port(port)
```

## Handling

Callers handle a result through `match`, `if let`, result combinators, or `try`.
There is no unchecked exception channel for ordinary application failure.

## Must-use behavior

- `Result` is always must-use.
- `Option` may be marked must-use by API context.
- Explicit discard uses `drop(value)` or `let _ = value` and can trigger a linter
  explanation requirement in strict projects.

## Error types

- Error types are ordinary enums, records, or newtypes.
- Public library errors should be structured and machine-matchable.
- Human messages are generated through `Display`.
- Source chains use the `Error` trait.
- Context may be attached explicitly without replacing the original source.

## Panic semantics

Edition 2026 panic aborts the process. It is not catchable and does not unwind.
This keeps control flow and cleanup behavior explicit. Recoverable situations must
not use panic.

Consequences:

- `Drop` and `defer` are guaranteed on normal and `Result`-based exits.
- They are not guaranteed after panic, process termination, or power loss.
- Servers should isolate fault domains through processes or supervised workers.

## Diagnostics

A failed `try` conversion, ignored result, impossible match, or invalid panic use
must produce a focused diagnostic with the source span and a safe fix when known.

## Exclusions

Edition 2026 has no checked/unchecked exception hierarchy, no hidden throws,
no catchable panic, and no implicit conversion from absence to error.
