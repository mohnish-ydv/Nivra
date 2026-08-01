# Source Manager

The `nivra-source` crate owns source text and location identity for one compiler
invocation.

## Guarantees

- every loaded file receives a stable `SourceId`
- every `Span` is a half-open byte range `[start, end)`
- invalid UTF-8 is rejected by source loading
- source slicing checks file identity, bounds, and UTF-8 boundaries
- line lookup uses a precomputed index
- user-facing line and column numbers are one-based
- columns count Unicode scalar values, not UTF-8 bytes
- CRLF and LF input are both represented correctly
- virtual files support tests, editors, and future REPL input

## Why byte spans

Compiler stages need cheap slicing and stable offsets. UTF-8 byte offsets provide
both, while the line map converts them into human positions only at the reporting
boundary.

## Deferred Unicode work

D2 intentionally deferred final identifier normalization. D3 supports Unicode
alphabetic identifiers and common combining marks, but public source stability
will require a versioned Unicode identifier and normalization policy before 1.0.
