# rqb

Ergonomic Postgres query builder for Rust services that need both hand-written queries and safe JSON-driven search.

rqb is not an ORM and not a Diesel clone. The main idea is:

1. Describe datasets with field metadata: API name, DB column, type, and allowed operations.
2. Build the trusted query shape in Rust: table/view/raw source, joins, CTEs, aggregates, locks, subqueries.
3. Optionally apply a client `SearchRequest` with selected fields, filters, sort, limit, and offset.
4. Render parameterized Postgres SQL or execute it through `tokio-postgres`/`deadpool-postgres`.

That gives a Knex-like runtime composition model without exposing arbitrary SQL in JSON.

## Install

```toml
[dependencies]
rqb = { version = "0.1", features = ["pool", "with-uuid", "with-chrono"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

Feature flags:

| Feature | Enables | Use when |
| --- | --- | --- |
| `runtime-tokio-postgres` | SQL params, execution traits, row deserialization | You own a `tokio_postgres::Client` or transaction |
| `runtime-deadpool` | `runtime-tokio-postgres` plus deadpool client support | You manage your own pool |
| `pool` | `Db`, `connect`, `begin`, transactions, savepoints | You want the built-in pool facade |
| `with-uuid` | `uuid::Uuid` values and native row reads | Your IDs are UUIDs |
| `with-chrono` | chrono date/time values and native row reads | Your structs use chrono |

Core query building and SQL rendering do not require a runtime feature.

## Quick Start

```rust
use chrono::{DateTime, Utc};
use rqb::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

const ID: Field = Field::new("id", FieldType::Uuid);
const EMAIL: Field = Field::new("email", FieldType::Text);
const STATUS: Field = Field::new("status", FieldType::Text);
const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamp);

fn users() -> Dataset {
    Dataset::table("app_users")
        .fields([ID, EMAIL, STATUS, CREATED_AT])
        .default_limit(50)
        .max_limit(500)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserRow {
    id: Uuid,
    email: String,
    status: String,
    created_at: DateTime<Utc>,
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb::connect("postgres://rqb:rqb@localhost:55432/rqb").await?;

    let rows = select(users())
        .filter(STATUS.eq("active"))
        .order_by(CREATED_AT.desc())
        .limit(20)
        .fetch_all_as::<UserRow>(&db)
        .await?;

    Ok(())
}
```

`rqb::connect` uses `NoTls` for local development. Cloud Postgres users can pass a `tokio-postgres` TLS connector with `rqb::connect_with_tls` or `rqb::postgres::Db::connect_with_max_size_and_tls`.

## Default Projection

You do not have to list fields for a normal `select(dataset())`. If `.fields(...)` is omitted, rqb selects every `selectable` field from the root dataset metadata:

```rust
let rows = select(users())
    .filter(STATUS.eq("active"))
    .fetch_all_as::<UserRow>(&db)
    .await?;
```

Use `.fields([...])` when you want a narrower response, qualified join columns, a one-column subquery, or a stable shape for dynamic JSON responses.

On joined queries, the default projection is still the root dataset only. Joined tables are available for filters, sort, aggregates, and explicit projections, but rqb does not silently return every joined column:

```rust
let rows = select(users().alias("u"))
    .left_join(orders().alias("o"), ID.on("u").eq_col(USER_ID.on("o")))
    .filter(STATUS.on("o").eq("paid"))
    .fetch_all_as::<UserRow>(&db)
    .await?;
```

This mirrors Diesel's usual ergonomics: a table query has a default selection, and `.select(...)` is only needed when changing the projection. Diesel tracks that selection in Rust types at compile time; rqb resolves it from dataset metadata at runtime.

## Why rqb Feels Different

For one static CRUD query, rqb and Diesel are close. The difference shows up when service code has optional filters, JSON search requests, nested response DTOs, and explicit async transaction boundaries:

```rust
let page = select(order_search_view())
    .filter(ORGANIZATION_ID.eq(current_org_id))
    .filter_option(params.status, |status| STATUS.eq(status))
    .filter_option(params.min_total, |min_total| TOTAL_CENTS.gte(min_total))
    .request(request)
    .page_as::<serde_json::Value>(&db)
    .await?;
```

The trusted query shape stays in Rust, while untrusted client search input is limited to metadata-approved fields and operators. See [docs/ergonomics.md](docs/ergonomics.md) for a Diesel-by-Diesel comparison with longer examples.

## SearchRequest

`SearchRequest` is the JSON API surface. It supports:

- `fields`
- `query`
- `sort`
- `limit`
- `offset`

Example HTTP body:

```json
{
  "fields": ["id", "email", "status", "totalCents"],
  "sort": [{ "field": "createdAt", "dir": "DESC" }],
  "query": {
    "logical": "and",
    "predicates": [
      { "field": "status", "operator": "equals", "value": "paid" },
      { "field": "metadata.score", "operator": "gte", "value": 80 }
    ]
  },
  "limit": 20,
  "offset": 0
}
```

Apply it to a server-owned query:

```rust
async fn search_orders(
    db: &Db,
    request: SearchRequest,
) -> rqb::Result<Page<serde_json::Value>> {
    select(order_search_view())
        .filter(ORGANIZATION_ID.eq(current_org_id()))
        .request(request)
        .page_as::<serde_json::Value>(db)
        .await
}
```

`.request(request)` merges the incoming request with existing builder state. Existing server filters are combined with the client filter using `AND`; request fields/sort/limit/offset replace those parts when present. Use `.replace_request(request)` only when you intentionally want old replacement semantics.

JSON does not define joins, CTEs, raw SQL, or subqueries. Build those in Rust, then apply `SearchRequest` on top.

## Server-Owned Query Shapes

### Joins

```rust
let users = Dataset::table("app_users").alias("u").fields([ID, EMAIL]);
let orders = Dataset::table("orders").alias("o").fields([ORDER_ID, USER_ID, STATUS]);

let rows = select(users)
    .left_join(orders, ID.on("u").eq_col(USER_ID.on("o")))
    .fields([ID.on("u"), EMAIL.on("u"), STATUS.on("o")])
    .filter(STATUS.on("o").eq("paid"))
    .fetch_all_as::<serde_json::Value>(&db)
    .await?;
```

Generated schemas include `table().alias("u")` relation helpers so most code can use `user.id()` and `order.user_id()` instead of string aliases.

### CTEs And Raw Sources

```rust
let recent = cte(
    "recent_orders",
    select(orders())
        .fields([ID, USER_ID, STATUS])
        .filter(CREATED_AT.gte("2026-01-01T00:00:00Z"))
        .build(),
);

let rows = select(Dataset::cte("recent_orders").fields([ID, USER_ID, STATUS]))
    .cte(recent)
    .filter(STATUS.eq("paid"))
    .fetch_all_as::<serde_json::Value>(&db)
    .await?;
```

Raw fragments are available for SQL shapes outside the builder:

```rust
let source = Dataset::raw(
    "SELECT id, email, total_cents FROM order_search_view WHERE total_cents > 0",
    "order_rollup",
)
.fields([ID, EMAIL, TOTAL_CENTS]);

select(source).request(request);
```

`Dataset::raw` is for static server-owned source SQL with declared fields. For bind values, use `raw("... ? ...").bind(value)` in filters, assignments, or CTE bodies. Use `raw_query("... ? ...").bind(value)` when the whole statement is hand-written SQL and should execute through `&Db`, `&Tx`, or another `&impl PgExecutor`. Use `??` for a literal question mark in raw SQL.

### Subqueries

Correlated `EXISTS`:

```rust
let rows = select(orders().alias("o"))
    .filter(exists(
        select(events().alias("e")).filter(all([
            EVENT_ORDER_ID.on("e").eq_col(ID.on("o")),
            EVENT_TYPE.on("e").eq("paid"),
        ])),
    ))
    .fetch_all_as::<OrderRow>(&db)
    .await?;
```

`IN (subquery)`:

```rust
let rows = select(users())
    .filter(ID.in_subquery(
        select(orders().alias("o"))
            .fields([USER_ID.on("o")])
            .filter(STATUS.on("o").eq("paid")),
    ))
    .fetch_all_as::<UserRow>(&db)
    .await?;
```

Subquery expressions are Rust-only and skipped by serde. This is deliberate: JSON clients should not author arbitrary subqueries.

## Postgres Features

### DISTINCT ON

```rust
select(orders())
    .fields([USER_ID, CREATED_AT, STATUS])
    .distinct_on([USER_ID])
    .order_by(USER_ID.asc())
    .order_by(CREATED_AT.desc());
```

Useful for "latest row per group" queries.

### Row Locks

```rust
let jobs = select(job_queue())
    .filter(STATUS.eq("ready"))
    .order_by(CREATED_AT.asc())
    .limit(100)
    .for_update()
    .skip_locked()
    .fetch_all_as::<Job>(&tx)
    .await?;
```

Available lock modes: `.for_update()`, `.for_no_key_update()`, `.for_share()`, `.for_key_share()`. Wait modes: `.nowait()`, `.skip_locked()`.

### Aggregates And Nested JSON

```rust
let rows = select(users().alias("u"))
    .left_join(orders().alias("o"), USER_ID.on("o").eq_col(ID.on("u")))
    .fields([ID.on("u"), EMAIL.on("u")])
    .agg(
        json_agg("orders", [ORDER_ID.on("o"), STATUS.on("o")])
            .filter(ORDER_ID.on("o").is_not_null())
    )
    .fetch_all_as::<UserWithOrders>(&db)
    .await?;
```

Other aggregates: `count`, `count_field`, `count_distinct`, `sum`, `avg`, `min`, `max`, `array_agg`, `json_agg`, and `string_agg`.

## Writes

Struct writes use serde.

```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct NewUser {
    id: Uuid,
    email: String,
    status: String,
}

let user = insert(users())
    .value(&new_user)
    .fetch_one_as::<UserRow>(&db)
    .await?;
```

Write `fetch_*` methods return all selectable fields by default. Use `.returning([ID, EMAIL])` to narrow the projection, or `.execute()` when no rows should be returned.

Writes can use SQL defaults, server-owned expressions, and computed `RETURNING` values:

```rust
let row = update(users())
    .set_default(PROFILE)
    .set_expr(EMAIL, func("lower", [EMAIL.expr()]).returns(FieldType::Text))
    .filter(ID.eq(user_id))
    .returning([ID])
    .returning_expr(func("lower", [EMAIL.expr()]).returns(FieldType::Text).alias("emailLower"))
    .fetch_one_as::<UserWriteResult>(&db)
    .await?;
```

Partial updates:

```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PatchUser {
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

update(users())
    .set_from(&patch)
    .filter(ID.eq(user_id))
    .returning([ID])
    .fetch_optional(&db)
    .await?;
```

Upsert:

```rust
insert(users())
    .value(&new_user)
    .on_conflict(EMAIL)
    .do_update([STATUS])
    .fetch_one_as::<UserRow>(&db)
    .await?;
```

## Transactions

Explicit begin/commit:

```rust
let tx = db.begin().await?;
insert(orders()).value(&new_order).execute(&tx).await?;
insert(order_items()).values(&items).execute(&tx).await?;
tx.commit().await?;
```

Dropping a transaction without `commit()` rolls it back. Closure-style code is available when it is a better fit:

```rust
db.transaction(txn!(|tx| {
    insert(orders()).value(&new_order).execute(tx).await?;
    insert(order_items()).values(&items).execute(tx).await?;
    Ok(())
}))
.await?;
```

## Debug SQL

```rust
let built = select(orders())
    .filter(STATUS.eq("paid"))
    .build_pg()?;

println!("{}", built.rows.debug_sql());
println!("{}", built.count.debug_sql());
```

`debug_sql()` prints SQL plus the `Value` params. It is for development logs, not string interpolation.

## Code Generation

Generate field constants from a live Postgres schema:

```bash
make db-up
cargo run -p rqb-cli -- generate \
  --database-url postgres://rqb:rqb@localhost:55432/rqb \
  --schema public \
  --out target/generated/rqb_schema.rs
```

The generated modules contain:

- `Field` constants
- `dataset()` functions
- `table().alias("x")` relation helpers for ergonomic joins
- Postgres enum metadata and serde-compatible Rust enum wrappers
- JSONB path policy and array sorting defaults from introspection

Generated enum wrappers serialize and deserialize as the exact Postgres labels, so they can be used directly in fixed-shape request, response, and DB DTOs.

The REST sample uses generated schema metadata directly:

```bash
make generate-demo
cargo run --manifest-path samples/rest-api/Cargo.toml
```

## Testing

```bash
make docker-test
```

This starts a dedicated Docker Compose project, puts Postgres data on tmpfs, runs the test suite in a Rust container, and tears the project down afterwards.

## Documentation Map

- [docs/guide.md](docs/guide.md): complete API guide
- [docs/recipes.md](docs/recipes.md): copyable service recipes
- [docs/testing.md](docs/testing.md): test layout, naming, rendering assertions, and integration pattern
- [docs/ergonomics.md](docs/ergonomics.md): longer comparison with Diesel from an application ergonomics angle
- [docs/diesel-migration.md](docs/diesel-migration.md): mapping Diesel patterns to rqb
- [docs/architecture.md](docs/architecture.md): internal capability spec and refactor direction
- [docs/roadmap.md](docs/roadmap.md): follow-up ergonomics ideas from the sample
- [PHILOSOPHY.md](PHILOSOPHY.md): project philosophy and API design principles
- [crates/rqb/examples](crates/rqb/examples): small compile-checked builder examples
- [samples](samples): standalone generated-schema samples for CRUD, JSON search, joins, transactions, CTEs, errors, raw SQL, and custom types
- [samples/rest-api](samples/rest-api): actix-web sample with generated schema, services, transactions, validation, and JSON search

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in rqb by you shall be dual licensed as above, without any additional terms or conditions.
