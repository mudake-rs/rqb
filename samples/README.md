# rqb Samples

Start here when reading the API.

- `basic-queries`: default projection, typed filters, sort, limit, and SQL rendering.
- `json-search`: server-owned query shape plus a safe JSON `SearchRequest`.
- `writes-and-types`: inserts, updates, deletes, raw SQL, exact numeric values,
  UUIDs, dates, timestamps, JSONB, and conflict handling.
- `transactions`: `tx!` plus explicit sqlx transaction control.
- `error-handling`: structured validation and database error matching.
- `raw-query`: raw SQL escape hatches that still validate bind counts.
- `joins-and-aggregates`: qualified joined fields, aggregates, and nested JSON.
- `cte-and-subqueries`: CTEs, recursive CTEs, lateral joins, set queries, and raw sources.
- `advanced-queries`: a larger server-owned query that combines the builder features.
- `custom-types`: raw-only schema metadata for database types outside the typed subset.
- `rest-api`: service-layer REST shape with pool execution, closure-style
  transactions, and JSON search.
- `schema`: shared generated schema crate used by the runnable samples.

The small samples build queries and assert rendered SQL. `rest-api` uses
`connect_lazy`, so it is compile-checked without a running database.

Sample comments call out the non-obvious pieces: default projections are
metadata-driven, alias handles remove repeated `.at("alias")` calls, raw sources
need exposed fields, and REST pagination stays in application code.

`samples/schema/src/lib.rs` is generated from `samples/schema.sql` by `rqb-cli`
and imported by the runnable samples:

```bash
make generate-sample-schema
```
