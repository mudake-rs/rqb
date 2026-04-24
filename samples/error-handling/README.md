# Error Handling

Shows structured validation and database error matching without parsing error
strings.

Execution mode: exercises validation and structured error values locally. The
database flow is compile-checked but not awaited.

## What This Shows

- Builder validation errors are returned before SQL rendering.
- sqlx database errors normalize into structured `rqb::Error` variants.
- API or service code can match `UniqueViolation`, `ForeignKeyViolation`,
  `NotFound`, and retryable transaction errors by meaning.
- The sample includes a compile-checked executed flow and points to
  `samples/rest-api/src/error.rs` for HTTP status mapping.

Run with:

```bash
cargo run --manifest-path samples/error-handling/Cargo.toml
```
