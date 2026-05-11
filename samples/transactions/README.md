# Transactions

Shows the preferred `tx!` closure pattern plus explicit sqlx `begin` / `commit`
for advanced transaction control.

Execution mode: renders one update statement and compile-checks the real
transaction flows without opening a database connection.

## What This Shows

- `tx!(&pool, |conn| { ... })` runs several async statements in one transaction.
- Service functions accept `impl PgExecutor<'e>`, so the same query works with a
  pool, connection, or transaction connection.
- The focused sample is compile-checked without a database, but the transaction
  function contains the real `.await?` flow used by applications.
- Explicit `pool.begin().await?` remains available when closure scope is not
  the right fit, for example when transaction ownership crosses helper
  boundaries or sqlx-native transaction controls are needed.

Run with:

```bash
cargo run --manifest-path samples/transactions/Cargo.toml
```
