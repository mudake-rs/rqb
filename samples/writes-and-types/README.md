# Writes And Types

Writes use typed field assignments. Any Rust type supported by sqlx for
Postgres can be bound through `Param::typed` or `Field<T>`.

Execution mode: renders SQL and asserts it. No database connection is opened.

## What This Shows

- `Insertable` maps DTO fields to generated schema fields without a serde JSON
  bridge.
- `Field<T>::set(...)` binds values through sqlx-supported Postgres types.
- `returning_all()` uses generated metadata for explicit `RETURNING` columns.
- `on_conflict(...).do_update_set(...)` keeps conflict target and update
  assignments structured.
- Numeric, temporal, JSONB, bytea, array, network, and range types stay on the
  sqlx encode path.

The schema comes from the shared `rqb-sample-schema` crate generated from
`../schema.sql`.

Run with:

```bash
cargo run --manifest-path samples/writes-and-types/Cargo.toml
```
