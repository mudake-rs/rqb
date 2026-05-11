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
- `constraint_name()` and `is_retryable()` cover common API boundary decisions
  without parsing database message strings.
- The sample includes a compile-checked executed flow; `samples/rest-api/src/error.rs`
  shows the same idea inside an axum `IntoResponse` mapping.

Run with:

```bash
cargo run --manifest-path samples/error-handling/Cargo.toml
```
