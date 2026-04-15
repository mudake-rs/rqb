# advanced-queries

Shows larger server-owned query shapes: CTEs, joins, lateral subqueries,
correlated predicates, JSON predicates, CASE, SQL functions, window functions,
aggregate filters, grouped JSON aggregation, and exact SQL parity tests.

Run from the repository root:

```bash
make db-up
cargo run --manifest-path samples/advanced-queries/Cargo.toml
cargo test --manifest-path samples/advanced-queries/Cargo.toml
```
