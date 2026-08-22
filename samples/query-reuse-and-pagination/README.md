# Query Reuse And Pagination

Focused sample for reusable query shapes, keyset pagination, and the JSON
search boundary.

Execution mode: renders SQL and asserts it without opening a database
connection.

## What This Shows

- Query-shape functions can return `Select` values that callers keep composing.
- Build once into `BuiltQuery` when the application wants to log, inspect, and
  execute the same validated SQL with the same bound params more than once;
  rebuild for different values.
- Default `select(table())` projection comes from generated schema metadata;
  callers add only the filters, ordering, and cursor predicates they own.
- Composite cursors use the same columns and direction as `ORDER BY`.
- UUIDv7 primary keys can work as id-only cursors when id order is the desired
  order.
- Full `SearchRequest` fits offset-style endpoints; cursor endpoints should
  accept a filter-only DTO so sort, limit, and offset stay server-owned.

Run with:

```bash
cargo run --manifest-path samples/query-reuse-and-pagination/Cargo.toml
```
