# Advanced Queries

Shows a larger server-owned query with CTEs, joins, lateral subqueries, window
functions, CASE, aggregate filters, and JSON aggregation.

Execution mode: renders SQL and asserts it. No database connection is opened.

## What This Shows

- A realistic report query can stay server-owned and still be built from typed
  pieces.
- CTEs, aliases, lateral joins, JSON aggregation, windows, CASE, `GROUP BY`,
  `HAVING`, and `FETCH WITH TIES` compose in one query.
- `case()` reads like SQL without exposing raw AST construction.
- `rqb::field!` gives computed columns metadata for later projection, grouping,
  and ordering.
- The sample is intentionally dense: use it as a capability stress test, not as
  the first tutorial.

Run with:

```bash
cargo run --manifest-path samples/advanced-queries/Cargo.toml
```
