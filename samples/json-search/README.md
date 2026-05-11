# JSON Search

`SearchRequest` accepts client filters, sort, limit, and offset. Rust still owns
the table, projection, joins, raw SQL, and server filters.

Execution mode: renders SQL and asserts it. No database connection is opened.

## What This Shows

- JSON requests compile into the same typed predicate AST as Rust builders.
- Server filters are preserved and combined with request filters using `AND`.
- Multiple sort keys are applied in client order after the server-owned filter.
- Unknown fields, hidden fields, bad operators, and bad JSON value shapes fail
  before rendering.
- Client input can control filter/sort/page, but not joins, projection, raw SQL,
  or subqueries.
- The sample asserts exact rendered SQL and parameter count without a database.

The schema comes from the shared `rqb-sample-schema` crate generated from
`../schema.sql`.

Run with:

```bash
cargo run --manifest-path samples/json-search/Cargo.toml
```
