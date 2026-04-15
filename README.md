# rqb

Ergonomic Postgres query builder for Rust services.

rqb is not an ORM. It builds parameterized Postgres SQL from dataset metadata,
validates query shapes before rendering, and can execute through
`tokio-postgres` or `deadpool-postgres`.

Use it when an application needs:

- normal Rust query builders for service-owned SQL shape
- safe JSON search over server-approved fields
- async execution with pool-or-transaction ergonomics
- serde row DTOs and small write DTOs
- generated schema metadata from Postgres

## TL;DR

Start with [`samples`](samples) if you want to see the API in real code. The
small samples each focus on one use case; [`samples/rest-api`](samples/rest-api)
shows the intended application shape with generated schema metadata, services,
transactions, validation, and JSON search.

## Status

rqb is pre-1.0 and the public API is still allowed to change when that makes the
library simpler or clearer.

## Install

```toml
[dependencies]
rqb = { version = "0.1", features = ["pool"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

Feature flags:

| Feature | Enables |
| --- | --- |
| `runtime-tokio-postgres` | execution traits, typed params, row deserialization |
| `runtime-deadpool` | `runtime-tokio-postgres` plus deadpool client support |
| `pool` | `Db`, `connect`, transactions, savepoints |

Query building and SQL rendering work without a runtime feature.

## Quick Start

```rust
use chrono::{DateTime, Utc};
use rqb::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

const ID: Field = Field::new("id", FieldType::Uuid);
const EMAIL: Field = Field::new("email", FieldType::Text);
const STATUS: Field = Field::new("status", FieldType::Text);
const CREATED_AT: Field = Field::new("created_at", FieldType::Timestamptz);

fn users() -> Dataset {
    Dataset::table("app_users")
        .fields([ID, EMAIL, STATUS, CREATED_AT])
        .default_limit(50)
        .max_limit(500)
}

#[derive(Deserialize)]
struct UserRow {
    id: Uuid,
    email: String,
    status: String,
    created_at: DateTime<Utc>,
}

async fn active_users(db: &Db) -> rqb::Result<Vec<UserRow>> {
    select(users())
        .filter(STATUS.eq("active"))
        .order_by(CREATED_AT.desc())
        .limit(20)
        .fetch_all_as::<UserRow>(db)
        .await
}
```

If `.fields(...)` is omitted, rqb selects every selectable field from the root
dataset. Joined fields must be selected explicitly.

## JSON Search

`SearchRequest` is the client-facing JSON shape. It can filter, sort, limit, and
offset. It cannot define joins, CTEs, raw SQL, subqueries, or response fields.

```json
{
  "filter": {
    "and": [
      { "field": "status", "operator": "equals", "value": "paid" },
      { "field": "metadata.score", "operator": "gte", "value": 80 }
    ]
  },
  "sort": [{ "field": "createdAt", "dir": "desc" }],
  "limit": 20,
  "offset": 0
}
```

Apply it to a trusted Rust query:

```rust
let page = select(order_search_view())
    .fields([ID, EMAIL, STATUS, TOTAL_CENTS])
    .filter(ORGANIZATION_ID.eq(current_org_id))
    .request(search_request)
    .page_as::<OrderSearchRow>(&db)
    .await?;
```

Server filters are preserved and combined with the client filter using `AND`.
The dataset metadata decides which fields and operators are valid.

## Writes

Writes can be built field-by-field:

```rust
insert(users())
    .set(ID, user_id)
    .set(EMAIL, "ada@example.com")
    .set(STATUS, "active")
    .returning([ID]);
```

For DTOs, derive `WriteRecord`:

```rust
mod user_fields {
    use rqb::prelude::*;

    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const EMAIL: Field = Field::new("email", FieldType::Text);
    pub const STATUS: Field = Field::new("status", FieldType::Text);
    pub const PROFILE: Field = Field::new("profile", FieldType::Jsonb);
}

fn writable_users() -> Dataset {
    Dataset::table("app_users").fields([
        user_fields::ID,
        user_fields::EMAIL,
        user_fields::STATUS,
        user_fields::PROFILE,
    ])
}

#[derive(rqb::WriteRecord)]
#[rqb(fields = user_fields)]
struct NewUser {
    id: uuid::Uuid,
    email: String,
    status: String,
    profile: serde_json::Value,
}

#[derive(rqb::WriteRecord)]
#[rqb(fields = user_fields, skip_none)]
struct UserPatch {
    status: Option<String>,
    profile: Option<serde_json::Value>,
}

insert(writable_users()).value(&new_user).execute(&db).await?;

update(writable_users())
    .set_from(&patch)
    .filter(user_fields::ID.eq(user_id))
    .execute(&db)
    .await?;
```

`#[rqb(skip_none)]` skips absent patch fields. Without it, `None` writes SQL
`NULL`. Use `#[rqb(field = user_fields::EMAIL)]` when a DTO field has a
different Rust name than the generated field constant, and `#[rqb(skip)]` for
request-only fields.

## SQL Shape

rqb supports Postgres-specific query shape in Rust:

- joins and lateral joins
- CTEs
- subquery sources and `EXISTS`
- `UNION`, `INTERSECT`, `EXCEPT`
- `DISTINCT ON`
- row locks
- aggregates and grouped queries
- `INSERT`, `UPDATE`, `DELETE`, upsert, `RETURNING`
- server-owned raw SQL fragments

Values are rendered as Postgres parameters. User input is not interpolated into
SQL strings.

## Raw SQL

Use raw fragments inside validated queries when the builder does not cover a
small expression:

```rust
select(users()).filter(raw("lower(email) = lower(?)").bind(email));
```

Use `raw_query` when the whole statement is hand-written SQL:

```rust
let count: i64 = raw_query("SELECT COUNT(*)::bigint FROM app_users WHERE status = ?")
    .bind("active")
    .fetch_one_scalar(&db)
    .await?;
```

`?` placeholders are counted and rendered as Postgres `$N` parameters. Use `??`
for a literal question mark.

## Transactions

```rust
let tx = db.begin().await?;

insert(users()).value(&new_user).execute(&tx).await?;
update(users()).set(STATUS, "active").filter(ID.eq(user_id)).execute(&tx).await?;

tx.commit().await?;
```

The pool feature also provides closure-style transactions through `txn!`:

```rust
db.transaction(txn!(|tx| {
    insert(users()).value(&new_user).execute(tx).await?;
    update(users())
        .set(STATUS, "active")
        .filter(ID.eq(user_id))
        .execute(tx)
        .await?;
    Ok(())
}))
.await?;
```

Savepoints are available from `Tx` when a transaction needs a smaller rollback
scope.

## Type Policy

`uuid`, `chrono`, JSON, arrays, enums, bytea, temporal types, ranges, network
types, and custom domains are modeled in field metadata.

`FieldType::Float` means Postgres `double precision`. `FieldType::Numeric` and
decimal-string custom domains keep exact values as strings by default, so large
money, balance, and domain values do not silently pass through `f64`.

## Code Generation

`rqb-cli` introspects a live Postgres schema and writes a Rust module with rqb
metadata. It is kept in this repository for now:

```bash
cargo run -p rqb-cli -- generate \
  --database-url postgres://rqb:rqb@localhost:55432/rqb \
  --schema public \
  --out src/schema.rs
```

Limit generation to specific tables or views with repeated `--table` flags:

```bash
cargo run -p rqb-cli -- generate \
  --database-url "$DATABASE_URL" \
  --schema public \
  --table app_users \
  --table orders \
  --out src/schema.rs
```

The generated module includes:

- `Field` constants
- `dataset()` functions
- schema-qualified table/view sources
- relation helpers for joins, such as `app_users::table().alias("u").email()`
- Rust enum wrappers for Postgres enums
- domain/custom type metadata
- JSONB path policy and array sorting defaults

Generation fails on unknown `--table` names and unsupported Postgres types
instead of guessing metadata.

Use generated metadata directly in queries:

```rust
use crate::schema::{app_users, enums::OrderStatus, orders};

let user = app_users::table().alias("u");
let order = orders::table().alias("o");

let rows = select(&user)
    .join(&order, user.id().eq_col(order.user_id()))
    .fields([user.id().alias("id"), user.email().alias("email")])
    .filter(order.status().eq(OrderStatus::Paid))
    .fetch_all_as::<UserOrderRow>(&db)
    .await?;
```

After changing the database schema, regenerate the module and commit the result.
The repository samples use this flow through `make generate-sample-base-schema`.

## Testing

```bash
cargo test --workspace --all-features
make docker-test
```

`make docker-test` starts the repository Postgres test container, runs the
workspace test suite, and tears the container down.

## Examples

- [`crates/rqb/examples`](crates/rqb/examples): small compile-checked examples
- [`samples`](samples): standalone samples for CRUD, JSON search, joins,
  transactions, CTEs, advanced builder queries, write DTOs, exact numerics,
  Postgres types, raw SQL, errors, and custom types
- [`samples/rest-api`](samples/rest-api): actix-web service sample

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
