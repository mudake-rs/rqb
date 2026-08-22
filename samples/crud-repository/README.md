# CRUD Repository

Experimental application-level sample for a tiny execution-only CRUD repository
macro plus a GAT-based `ExecutorSource` wrapper around sqlx executors.

Execution mode: compile-checks pool and transaction flows without opening a
database connection.

## What This Shows

- rqb query shapes can be wrapped in local repository-style APIs when an
  application wants that convention.
- The macro is sample-local glue over normal rqb builders and exposes executing
  methods, not reusable query objects. Use normal rqb query-shape helpers when
  composition is the goal.
- The macro is not an ORM layer and it does not add runtime reflection.
- The macro assumes this module's imports (`rqb::prelude::*` and `uuid::Uuid`);
  it is written for readability, not for publishing as a reusable macro crate.
- Empty changesets return `Ok(None)` before building SQL, because `PATCH {}`
  should be an application decision instead of an accidental empty `UPDATE`.
- `ExecutorSource` hides sqlx's executor lifetime for multi-statement helpers
  that need to reborrow the same pool, connection, or transaction source more
  than once.
- One-statement helpers can still accept plain `impl PgExecutor<'_>`; use the
  wrapper only where repeated reborrows make the call site clearer.
- `ExecutorSource` is only lifetime/reborrow glue. Calling a multi-statement
  helper with `&PgPool` may acquire separately per statement; pass a
  transaction when the operations must commit or roll back together.

Run with:

```bash
cargo run --manifest-path samples/crud-repository/Cargo.toml
```
