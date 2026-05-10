# rqb REST API Sample

Compile-checked REST service shape for the sqlx-first API.

Execution mode: builds the router and service layer with `connect_lazy`. It
does not start a server or open a database connection by default.

It is intentionally closer to a production backend than a CRUD toy. The routes
cover patterns that tend to decide whether a query builder is pleasant in real
service code:

- `PATCH /users/{id}` uses `#[derive(Changeset)]`, so `Option<T>::None`
  leaves a column untouched.
- `GET /orders` uses seek/cursor pagination with Postgres row comparison:
  `row((created_at, id)).lt((cursor_created_at, cursor_id))`.
- `POST /orders/{id}/transition` locks the order row, checks a state machine,
  updates it, and writes an audit event in one `tx!` transaction.
- `POST /products/upsert` uses `Insertable` plus
  `do_update_excluded((name, price_cents, attributes, tags))`.
- `GET /reports/orders-by-day` shows aggregate report code with `date_trunc`,
  `GROUP BY`, `sum`, and `count_all`.
- `GET /orders/export.csv` streams Postgres rows into HTTP response chunks with
  `BuiltQuery::fetch_stream_as` and `Body::from_stream`.
- `POST /orders/search` applies JSON `SearchRequest` only after the service has
  installed server-owned filters.

## What This Shows

- Handlers own HTTP validation and response shaping.
- Services own database query shape and accept `impl PgExecutor<'e>` where they
  should be reusable with a pool or transaction connection.
- Write DTOs derive `Insertable` or `Changeset` directly when the public request
  shape and the database write shape match. Split them only when they diverge.
- REST pagination is application code: `limit`, `offset`, and `Select::count()`
  for page-style endpoints; cursor pagination for large ordered lists.
- `ApiError` maps structured rqb errors to HTTP responses without parsing
  database message strings.
- Generated alias handles and typed fields keep joins and writes readable
  without hiding the underlying SQL shape.

The sample uses `PgPoolOptions::connect_lazy`, so `cargo check` does not need a
running database.

Regenerate the shared sample schema crate from the sample SQL:

```bash
make generate-sample-schema
```

Run the compile-checked router bootstrap with:

```bash
cargo run --manifest-path samples/rest-api/Cargo.toml
```
