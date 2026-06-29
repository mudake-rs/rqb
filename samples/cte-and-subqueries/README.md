# CTE And Subqueries

Shows CTEs, recursive CTEs, `EXISTS`, `IN (subquery)`, scalar subqueries,
lateral joins, set queries, and raw sources with field metadata.

Execution mode: renders SQL and asserts it. No database connection is opened.

## What This Shows

- `infer_cte(...)` infers fields from plain projections.
- `rqb::field!` defines metadata for computed columns, and source helpers
  accept fields directly instead of manual `*field.meta` vectors.
- `scalar_subquery(...)` turns a server-owned select into a value expression for
  comparisons.
- Recursive CTEs use `cte(...).recursive()` for the `WITH RECURSIVE` wrapper;
  raw recursive bodies should not include `WITH RECURSIVE` themselves.
- Tree traversal row structs use `Option<T>` for nullable parent links while
  field metadata keeps the underlying SQL type.
- Common set-returning functions such as `generate_series` and `unnest` can be
  exposed as typed sources without raw SQL.
- Lateral joins and raw sources declare exposed fields for outer typed queries.
- `values_source(...)` builds `FROM (VALUES ...) AS alias(columns...)` with
  explicit field metadata.
- Set queries such as `UNION` preserve parameter ordering across both sides.

Run with:

```bash
cargo run --manifest-path samples/cte-and-subqueries/Cargo.toml
```
