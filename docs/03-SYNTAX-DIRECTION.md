# Syntax Direction v0.1

This document defines the intended surface style, not the complete grammar. Exact
type, memory, ABI, and concurrency semantics are locked in the next design delivery.

## Design goals

- familiar to developers from C-family languages
- readable on narrow phone screens
- minimal punctuation without indentation-sensitive blocks
- one obvious form for common operations
- explicit mutation, absence, fallibility, concurrency, and unsafe behavior
- friendly to deterministic formatting and parser recovery

## Source format

- UTF-8 source
- working file extension: `.trn`
- braces delimit blocks
- semicolons are optional at line endings
- semicolons may separate multiple statements on one line, but the formatter expands them
- `//` line comments
- `/* ... */` nested block comments
- documentation comments use `///`
- identifiers use Unicode letters, digits after the first character, and `_`
- public API identifiers should use ASCII unless a project explicitly opts out

## Naming convention

- types and traits: `PascalCase`
- functions, variables, modules, fields: `snake_case`
- constants: `SCREAMING_SNAKE_CASE`
- packages: lowercase words separated by `-`
- no style significance in the compiler; formatter/linter enforce the official convention

## Modules and imports

```trion
module weather.client

use std.http.Client
use std.time.Duration
use app.models.{Forecast, City}
```

Files belong to one module. Imports are explicit. Wildcard imports are rejected
from public library code by the linter.

## Bindings

```trion
let port = 8080
let host: String = "127.0.0.1"

var attempts = 0
attempts += 1
```

`let` is immutable. `var` is mutable. Type inference never changes the public API
surface silently; exported declarations require explicit types unless trivially
derivable under a future specified rule.

## Primitive direction

```trion
let enabled: Bool = true
let count: Int = 42
let ratio: Float = 0.75
let letter: Char = 'A'
let message: String = "hello"
```

Exact integer widths, overflow behavior, decimal types, and literal inference are
deferred to D2.

## String interpolation

```trion
let greeting = "Hello, ${user.name}"
```

Interpolation is explicit and formatted through a standard display capability.

## Functions

```trion
fn add(a: Int, b: Int) -> Int {
    a + b
}

fn clamp(value: Int, min: Int = 0, max: Int = 100) -> Int {
    if value < min { min }
    else if value > max { max }
    else { value }
}
```

Blocks are expressions. `return` remains available for early exit. Named arguments
are supported where clarity matters:

```trion
let client = connect(
    host: "api.example.com",
    timeout: 5.seconds,
    retries: 3,
)
```

## Control flow

```trion
if temperature < 0 {
    print("freezing")
} else {
    print("above freezing")
}

for item in items {
    process(item)
}

while queue.is_not_empty() {
    process(queue.pop())
}
```

No general truthiness exists. Conditions require `Bool`.

## Records and structs

Use `record` for ordinary data:

```trion
record User {
    id: UserId
    name: String
    email: String? = none
}
```

A record may derive standard behavior without handwritten constructors, equality,
copy, debug output, or serialization glue:

```trion
@derive(Debug, Equal, Json)
record Point {
    x: Float
    y: Float
}
```

Use `struct` only when representation, mutability, or low-level layout matters:

```trion
@repr(C)
struct NativePoint {
    x: F32
    y: F32
}
```

The exact derive mechanism and layout rules are deferred to the full specification.

## Enums and pattern matching

```trion
enum LoadState<T> {
    idle
    loading
    ready(T)
    failed(AppError)
}

match state {
    .idle => show_idle()
    .loading => show_spinner()
    .ready(value) => render(value)
    .failed(error) => show_error(error)
}
```

Matches over closed enums must be exhaustive.

## Absence

```trion
let nickname: String? = none

if let name = nickname {
    print(name)
} else {
    print("No nickname")
}
```

There is no ambient `null`. Optional chaining syntax is not locked in D1.

## Recoverable errors

```trion
fn load_config(path: Path) -> Result<Config, ConfigError> {
    let text = try fs.read_text(path)
    let raw = try json.parse(text)
    Config.from_json(raw)
}
```

`try` propagates a compatible error. Ignoring a `Result` requires a deliberate
operation. Unchecked exceptions are not routine application control flow.

## Traits and implementation

```trion
trait Display {
    fn display(self) -> String
}

impl Display for User {
    fn display(self) -> String {
        "${self.name} #${self.id}"
    }
}
```

V1 does not use class inheritance. Reuse comes from composition, traits, generic
constraints, and ordinary functions.

## Generics

```trion
fn first<T>(items: List<T>) -> T? {
    if items.is_empty() { none } else { items[0] }
}

fn render<T: Display>(value: T) {
    print(value.display())
}
```

Generic constraints are explicit. Specialization and variance rules are deferred.

## Resource cleanup

```trion
fn copy_file(source: Path, target: Path) -> Result<Void, IoError> {
    let input = try File.open_read(source)
    defer input.close()

    let output = try File.open_write(target)
    defer output.close()

    try stream.copy(input, output)
}
```

`defer` communicates deterministic cleanup. Its interaction with ownership and
automatic resource types will be specified in D2.

## Concurrency direction

```trion
async fn fetch_all(urls: List<Url>) -> Result<List<Response>, NetError> {
    task_group {
        for url in urls {
            spawn fetch(url)
        }
    }
}
```

This is directional syntax only. Cancellation, ordering, error aggregation, data
sharing, and scheduling semantics are deferred.

## Unsafe boundary

```trion
unsafe {
    let pointer = memory.allocate(bytes: 128)
    native_write(pointer)
}
```

Unsafe code cannot be introduced by inference. The final model will use named
capabilities so an audit can explain why each unsafe operation is allowed.

## Visibility

```trion
pub record Account {
    pub id: AccountId
    balance: Money
}

pub fn open_account(owner: Person) -> Result<Account, AccountError> {
    // ...
}
```

Declarations are module-private by default.

## Conversion policy

Implicit lossy conversions are rejected. The intended forms are explicit:

```trion
let count: I64 = value.to_i64()
let port: U16 = try raw.try_to_u16()
```

Exact APIs and overflow behavior are deferred.

## Rejected syntax directions

- preprocessor directives
- header/source file duplication
- class inheritance syntax
- implicit truthiness
- ambient null
- unchecked exception declarations
- pointer arithmetic outside unsafe code
- mandatory semicolons
- indentation-sensitive blocks
- unrestricted operator overloading
- unrestricted textual macros
