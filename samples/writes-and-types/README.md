# Writes And Types

Writes use typed field assignments. Any Rust type supported by sqlx for
Postgres can be bound through `Param::typed` or `Field<T>`.

Execution mode: renders SQL and asserts it. No database connection is opened.

## What This Shows

- `Insertable` maps DTO fields to generated schema fields without a serde JSON
  bridge.
- `Changeset` maps PATCH DTOs to assignments and skips `None` fields.
- `Field<T>::set(...)` binds values through sqlx-supported Postgres types;
  `set_many((...))` batches manual assignments without losing field metadata.
- Generated PostgreSQL enums behave like normal typed fields in Rust builder
  predicates.
- `Field<T>::set_null()` writes SQL `NULL` explicitly when application state
  needs to clear a nullable column.
- `Field<T>::set_default()` delegates column defaults to PostgreSQL; use
  `insert(...).default_values()` only for tables whose omitted columns are all
  nullable or defaulted.
- `set_if(...)` and `set_option(...)` keep conditional assignments in the
  builder chain.
- `returning_as(field.old_value(), ...)` and `new_value()` render PostgreSQL 18
  old/new DML returning fields.
- `returning((...))` accepts a compact tuple/list of fields for write response
  shapes.
- `returning_all()` uses generated metadata for explicit `RETURNING` columns.
- Column conflict targets can carry predicates through `.target_where(...)`,
  then use `do_update_set_where(...)` for a filtered `DO UPDATE`.
- `do_update_excluded((...))` updates several fields from `EXCLUDED` without
  repeating per-field `.set_excluded()` calls.
- `merge_into(...)` handles relation-driven matched updates and not-matched
  inserts without dropping to raw SQL.
- `values_many(...)` keeps DTO batch upserts from repeating target columns;
  `set_from("alias")` copies update values from the incoming source.
- `UPDATE ... FROM` and `DELETE ... USING` compose with typed aliases and CTEs.
- Optimistic compare-and-swap writes are normal updates with the expected
  current state in `WHERE` and `RETURNING` for success detection.
- Generated constraint name constants feed `on_conflict_constraint(...)`.
- `insert(...).from_select((...), select(...))` validates target column count
  against the server-owned select projection.
- Formatting and range helpers such as `to_char`, `range_lower`, and `isempty`
  build common report columns without raw SQL.
- Numeric, temporal, JSONB, and interval-shaped values stay on the sqlx encode
  path.

The schema comes from the shared `rqb-sample-schema` crate generated from
`../schema.sql`.

Run with:

```bash
cargo run --manifest-path samples/writes-and-types/Cargo.toml
```
