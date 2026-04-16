# Joins And Aggregates

Shows qualified joined fields, grouped aggregates, `DISTINCT ON`, and nested
JSON aggregation.

## What This Shows

- `let u = users::alias("u")` removes repeated string aliases in joins.
- `jsonb_agg_object![o.id(), o.status()]` derives JSON keys from field
  metadata.
- Aggregate `FILTER` and aggregate-local `ORDER BY` stay in the builder.
- `DISTINCT ON`, grouped aggregates, and `HAVING` render as ordinary Postgres
  SQL.
- This is the best short sample for seeing why rqb is more than string
  concatenation.

Run with:

```bash
cargo run --manifest-path samples/joins-and-aggregates/Cargo.toml
```
