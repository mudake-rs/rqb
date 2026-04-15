# Error handling

Small error-handling sample:

- `optional()` turns a missing row into `Option<T>`
- invalid field names fail validation before SQL is sent
- duplicate keys return `rqb::Error::UniqueViolation`
- missing parents return `rqb::Error::ForeignKeyViolation`
- `code()` and `constraint_name()` are available when logs or API mapping need
  database details

Run from the repository root:

```bash
make db-up
cargo run --manifest-path samples/error-handling/Cargo.toml
```
