# rqb Samples

Start here when reading the API. The samples are compile-checked; most focused
samples also assert the SQL they render, so they double as executable API
documentation.

## Start Here

1. **See the difference**: [`joins-and-aggregates`](joins-and-aggregates/)
   shows alias handles, aggregate `FILTER`, and `jsonb_agg_object!` in one
   realistic report query.
2. **Real service shape**: [`rest-api`](rest-api/) shows routes, DTOs,
   services, `PgExecutor`, `tx!`, JSON search, and application pagination.
3. **Simple first**: [`basic-queries`](basic-queries/) shows typed filters,
   default projection, ordering, and bound parameters.

## Catalog

- [`basic-queries`](basic-queries/): default projection, typed filters, sort,
  limit, and SQL rendering.
- [`json-search`](json-search/): server-owned query shape plus safe client
  filters, sort, limit, and offset.
- [`writes-and-types`](writes-and-types/): inserts, updates, deletes, exact
  numeric values, UUIDs, dates, timestamps, JSONB, arrays, and conflict
  handling.
- [`transactions`](transactions/): `tx!` plus explicit sqlx transaction control.
- [`error-handling`](error-handling/): structured validation and database error
  matching.
- [`raw-query`](raw-query/): raw SQL escape hatches that still validate bind
  counts.
- [`joins-and-aggregates`](joins-and-aggregates/): qualified joined fields,
  grouped aggregates, `DISTINCT ON`, and nested JSON.
- [`cte-and-subqueries`](cte-and-subqueries/): CTEs, recursive CTEs, lateral
  joins, set queries, and raw sources.
- [`advanced-queries`](advanced-queries/): a larger server-owned query that
  combines CTEs, joins, lateral subqueries, windows, CASE, aggregate filters,
  and JSON aggregation.
- [`custom-types`](custom-types/): raw-only schema metadata for extension types
  outside the typed subset.
- [`rest-api`](rest-api/): service-layer REST shape with pool execution,
  closure-style transactions, and JSON search.
- [`schema`](schema/): shared generated schema crate used by the runnable
  samples.

## Running Against A Database

The focused samples avoid real connections and either assert rendered SQL or
exercise validation/error paths. `rest-api` uses `connect_lazy`, so it is
compile-checked without a running database.

To run service code against a real Postgres instance, start the sample database
and use the same schema crate:

```bash
make db-up
DATABASE_URL=postgres://rqb:rqb@localhost:55432/rqb cargo run --manifest-path samples/rest-api/Cargo.toml
```

The focused samples intentionally do not connect: they keep CI fast and make
the rendered SQL visible. `rest-api` shows the full executed service pattern.

Sample comments call out the non-obvious pieces: default projections are
metadata-driven, alias handles remove repeated `.at("alias")` calls, raw sources
need exposed fields, and REST pagination stays in application code.

`samples/schema/src/lib.rs` is generated from `samples/schema.sql` by `rqb-cli`
and imported by the runnable samples:

```bash
make generate-sample-schema
```
