# rqb Samples

Start here when reading the API. The samples are compile-checked, and the
focused query samples also assert the SQL they render, so they double as
executable API documentation.

## Start Here

1. **Simple first**: [`basic-queries`](basic-queries/) shows typed filters,
   default projection, ordering, and bound parameters.
2. **See the difference**: [`joins-and-aggregates`](joins-and-aggregates/)
   shows alias handles, aggregate `FILTER`, and `jsonb_agg_object!` in one
   realistic report query.
3. **Real service shape**: [`rest-api`](rest-api/) shows routes, DTOs,
   services, `PgExecutor`, `tx!`, cursor pagination, upserts, JSON search, and
   streamed CSV responses.

## Reading Order

- [`basic-queries`](basic-queries/): core typed reads. Renders SQL and asserts
  it.
- [`json-search`](json-search/): client-controlled filters on top of
  server-owned query shape. Renders SQL and asserts it.
- [`query-reuse-and-pagination`](query-reuse-and-pagination/): reusable query
  shapes, keyset cursors, UUIDv7-style id cursors, and the JSON search cursor
  boundary. Renders SQL and asserts it.
- [`writes-and-types`](writes-and-types/): inserts, updates, deletes, write
  DTOs, conflict handling, and sqlx-backed Postgres types. Renders SQL and
  asserts it.
- [`joins-and-aggregates`](joins-and-aggregates/): joins, grouped aggregates,
  `DISTINCT ON`, and nested JSON. Renders SQL and asserts it.
- [`cte-and-subqueries`](cte-and-subqueries/): CTEs, recursive CTEs, lateral
  joins, set queries, raw sources, set-returning function sources, and
  `VALUES` sources. Renders SQL and asserts it.
- [`advanced-queries`](advanced-queries/): denser server-owned report query
  combining CTEs, joins, windows, CASE, and JSON aggregation. Renders SQL and
  asserts it.
- [`raw-query`](raw-query/): raw SQL escape hatches that still validate bind
  counts. Renders SQL and asserts it.
- [`error-handling`](error-handling/): validation errors and normalized
  database errors. No database connection required.
- [`transactions`](transactions/): `tx!` plus explicit sqlx transaction
  control. Renders SQL and compile-checks transaction flows without connecting.
- [`custom-types`](custom-types/): raw-only schema metadata for extension types
  outside the typed subset. Renders SQL and asserts it.
- [`crud-repository`](crud-repository/): sample-local execution-only CRUD
  repository macro plus a GAT `Db` wrapper. Compile-checks pool/transaction
  flows without connecting.
- [`rest-api`](rest-api/): service-layer REST shape with pool execution,
  closure-style transactions, cursor pagination, aggregate reports, streaming
  export, and JSON search. Builds the router without listening or connecting.
- [`schema`](schema/): shared generated schema crate used by the runnable
  samples.

## Experiments

- [`executor-wrapper`](executor-wrapper/): optional design probe for a GAT
  wrapper around sqlx executors. It is compile-checked, but it is not the
  recommended service style unless the wrapper proves more ergonomic than
  `&PgPool`, reusable query-shape helpers, and targeted `PgExecutor<'_>`
  helpers.
- [`crud-repository`](crud-repository/): optional design probe for a
  macro-built executing repository on top of normal rqb builders and the same
  GAT executor wrapper idea.

## Running Against A Database

The focused samples avoid real connections and either assert rendered SQL or
exercise validation and error paths. `rest-api` uses `connect_lazy`, so it is
compile-checked without a running database.

To run service code against a real Postgres instance, start the sample database
and use the same schema crate:

```bash
make db-up
DATABASE_URL=postgres://rqb:rqb@localhost:55432/rqb cargo run --manifest-path samples/rest-api/Cargo.toml
```

The focused samples intentionally do not connect: they keep CI fast and keep
the rendered SQL visible. `rest-api` shows the full service shape, while the
focused samples stay small enough to read in one sitting.

Sample comments call out the non-obvious pieces: default projections are
metadata-driven, alias handles remove repeated `.at("alias")` calls, raw sources
need exposed fields, REST pagination stays in application code, and pool-owned
stream helpers keep HTTP body streams independent of the handler call frame.

`samples/schema/src/lib.rs` is generated from `samples/schema.sql` by `rqb-cli`
and imported by the runnable samples:

```bash
make generate-sample-schema
```
