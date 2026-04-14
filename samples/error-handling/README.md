# error-handling

Shows common app error patterns:

- map named constraints to application errors with `on_constraint`
- map `NotFound` to `Option` with `optional`
- retry a serializable transaction when `is_retryable` says the error can be retried
- surface validation errors before SQL is sent to Postgres

Run from the repository root:

```bash
make db-up
cargo run --manifest-path samples/error-handling/Cargo.toml
```
