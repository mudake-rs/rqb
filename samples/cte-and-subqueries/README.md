# CTE And Subqueries

Shows CTEs, recursive CTEs, `EXISTS`, `IN (subquery)`, scalar subqueries,
lateral joins, set queries, and raw sources with field metadata.

Execution mode: renders SQL and asserts it. No database connection is opened.

## What This Shows

- `try_into_cte(...)` infers fields from plain projections.
- `rqb::field!` defines metadata for computed columns, and source helpers
  accept fields directly instead of manual `*field.meta` vectors.
- `scalar_subquery(...)` turns a server-owned select into a value expression for
  comparisons.
- Recursive CTEs can use raw SQL while still validating bind counts.
- Lateral joins and raw sources declare exposed fields for outer typed queries.
- Set queries such as `UNION` preserve parameter ordering across both sides.

Run with:

```bash
cargo run --manifest-path samples/cte-and-subqueries/Cargo.toml
```
