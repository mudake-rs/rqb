# rqb Samples

Start here when reading the API.

- `basic-queries`: typed fields, filters, sort, limit, and SQL rendering.
- `json-search`: server-owned query shape plus a safe JSON `SearchRequest`.
- `rest-api`: service-layer REST shape with pool execution, explicit
  transactions, closure-style transactions, and JSON search.
- `schema`: shared generated schema crate used by the runnable samples.
- `writes-and-types`: inserts, updates, deletes, raw SQL, exact numeric values,
  UUIDs, dates, timestamps, and JSONB.

The small samples build queries and assert rendered SQL. `rest-api` uses
`connect_lazy`, so it is compile-checked without a running database.

`samples/schema/src/lib.rs` is generated from `samples/schema.sql` by `rqb-cli`
and imported by the runnable samples:

```bash
make generate-sample-schema
```
