# Executor Wrapper

Experimental sample for a GAT-based local wrapper around sqlx executors.

Execution mode: renders one query and compile-checks pool, transaction, and
connection flows without opening a database connection.

## What This Shows

- For one-statement helpers, plain `impl PgExecutor<'_>` is still the smaller
  API: no local trait, no mutable parameter, no `.exec()` calls.
- A small GAT trait can hide sqlx's executor lifetime from service function
  signatures when a helper needs to execute more than one statement from the
  same source.
- Implementations for `&PgPool`, `&mut PgConnection`, `&mut PoolConnection`,
  and `&mut Transaction` can centralize some sqlx reborrow ceremony.
- The main caveat is atomicity: a multi-statement helper called with `&PgPool`
  can acquire independently per statement, and an acquired connection is still
  not a transaction. Use a transaction when the statements must commit or roll
  back together.
- The tradeoff is real: every service function now calls `db.exec()`, and the
  wrapper becomes a new abstraction to document, name, and keep stable.
- This is a sample-local experiment, not a recommended rqb core API yet.

Run with:

```bash
cargo run --manifest-path samples/executor-wrapper/Cargo.toml
```
