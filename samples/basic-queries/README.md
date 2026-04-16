# Basic Queries

Typed fields build parameterized Postgres SQL. Result structs are normal sqlx
rows when the query is executed.

`select(table())` uses the table metadata as the default projection; explicit
`.column(...)` calls are only needed for subsets, aliases, joins, and
expressions.

The schema comes from the shared `rqb-sample-schema` crate generated from
`../schema.sql`.

Run with:

```bash
cargo run --manifest-path samples/basic-queries/Cargo.toml
```
