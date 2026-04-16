# Basic Queries

Typed fields build parameterized Postgres SQL. Result structs are normal sqlx
rows when the query is executed.

## What This Shows

- `select(users::table())` renders known metadata fields, not `SELECT *`.
- `.filter(...)` composes typed predicates and binds Rust values as `$N`.
- `.filter_if(...)` and `.filter_option(...)` keep optional filters readable.
- `.column(...)` narrows projection without string column names.
- `.order_asc_nulls_last(...)`, `.limit(...)`, and `.offset(...)` stay in the
  typed builder.

`select(table())` uses the table metadata as the default projection; explicit
`.column(...)` calls are only needed for subsets, aliases, joins, and
expressions.

The schema comes from the shared `rqb-sample-schema` crate generated from
`../schema.sql`.

Run with:

```bash
cargo run --manifest-path samples/basic-queries/Cargo.toml
```
