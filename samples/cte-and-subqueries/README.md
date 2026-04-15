# cte-and-subqueries

Shows advanced query composition from simpler subqueries to escape hatches:
`EXISTS`, `IN (subquery)`, `UNION`, typed CTEs, lateral joins, and raw SQL
sources with field metadata.

Run from the repository root:

```bash
make db-up
cargo run --manifest-path samples/cte-and-subqueries/Cargo.toml
```
