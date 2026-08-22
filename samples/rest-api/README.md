# rqb REST API Sample

Compile-checked REST service shape for the sqlx-first API.

Execution mode: builds the router and service layer with `connect_lazy`. It
does not start a server or open a database connection by default.

It is intentionally closer to a production backend than a CRUD toy. The routes
cover patterns that tend to decide whether a query builder is pleasant in real
service code:

- `PATCH /users/{id}` uses `#[derive(Changeset)]`, so `Option<T>::None`
  leaves a column untouched. Empty PATCH bodies are rejected at the HTTP
  boundary before rqb builds an empty `UPDATE`.
- `GET /orders` uses seek/cursor pagination with Postgres row comparison:
  `row((created_at, id)).lt((cursor_created_at, cursor_id))`.
- `GET /orders/filter` parses raw query-string values into typed filters at the
  HTTP boundary before calling the database service.
- `POST /orders/{id}/transition` locks the order row, checks a state machine,
  updates it, and writes an audit event in one `tx!` transaction.
- `POST /products/upsert` uses `Insertable` plus
  `do_update_excluded((name, price_cents, attributes, tags))`.
- `GET /reports/orders-by-day` shows aggregate report code with `date_trunc_part`,
  `GROUP BY`, `sum`, and `count_all`.
- `GET /orders/summary` builds a `jsonb_agg_object!` child collection and maps
  it into `#[sqlx(json)] Vec<OrderSummaryItem>`.
- `GET /orders/export.csv` streams Postgres rows into HTTP response chunks with
  `fetch_stream_pool_as` and `Body::from_stream`.
- `POST /orders/search` applies JSON `SearchRequest` only after the service has
  installed server-owned filters and bounded application page limits.

## What This Shows

- Handlers own HTTP validation and response shaping.
- Services own database query shape. Route-only services accept `&PgPool`;
  `users::find_query` shows reusable query shape inside `tx!`; small helpers
  that should execute directly in either context use `impl PgExecutor<'_>`.
- Write DTOs derive `Insertable` or `Changeset` directly when the public request
  shape and the database write shape match. Split them only when they diverge.
- REST pagination is application code: `limit`, `offset`, and `Select::count()`
  for page-style endpoints; cursor pagination for large ordered lists. The
  service clamps page limits and installs a deterministic default order when
  clients omit sort keys.
- Query-string parsing stays in axum/serde structs before service calls; bad
  `min_total`, `from_date`, or `limit` values fail before rqb sees typed Rust
  values.
- JSON extractor failures also happen before rqb sees `SearchRequest`; production
  APIs can map axum's rejection type when they need the same error envelope.
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
