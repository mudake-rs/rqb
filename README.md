# rqb

sqlx-first Postgres query builder for Rust services.

rqb is not an ORM. It builds parameterized SQL from server-owned query shape and
small field metadata. Typed Rust values go straight into sqlx bind arguments;
JSON search is a constrained adapter for filters, sort, limit, and offset.

## TL;DR

Start with [`samples`](samples). They are short, compile-checked, and show the
actual API faster than prose.

## Status

Pre-public, pre-1.0. APIs are still allowed to break when that makes the library
simpler or clearer.

## Install

```toml
[dependencies]
rqb = "0.1"
chrono = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", features = [
    "postgres",
    "derive",
    "uuid",
    "chrono",
    "json",
    "runtime-tokio-rustls",
] }
uuid = "1"
```

`uuid`, `chrono`, JSON, numeric, ranges, arrays, and other Postgres values are
accepted when the Rust type implements sqlx `Encode` and `Type` for Postgres.

## Basic Query

```rust
use rqb::prelude::*;
use uuid::Uuid;

static ID_META: Meta = Meta::new("id", "id", "uuid")
    .ops(OpSet::ordered())
    .json(JsonKind::Uuid);
static EMAIL_META: Meta = Meta::new("email", "email", "text")
    .ops(OpSet::ordered())
    .json(JsonKind::Text);
static STATUS_META: Meta = Meta::new("status", "status", "text")
    .ops(OpSet::ordered())
    .json(JsonKind::Text);

const ID: Field<Uuid> = Field::new(&ID_META);
const EMAIL: Field<String> = Field::new(&EMAIL_META);
const STATUS: Field<String> = Field::new(&STATUS_META);

static USER_FIELDS: [&Meta; 3] = [&ID_META, &EMAIL_META, &STATUS_META];

fn users() -> Source {
    rqb::table("public.app_users", &USER_FIELDS)
}

let query = select(users())
    .column(ID)
    .column(EMAIL)
    .filter(STATUS.eq("active"))
    .order_asc(EMAIL)
    .limit(20)
    .build()?;
```

`query.sql` contains `$N` placeholders and `query.arguments()?` creates
`sqlx::postgres::PgArguments` at execution time.

## Execution

Use any sqlx executor:

```rust
#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
}

let rows = select(users())
    .column(ID)
    .column(EMAIL)
    .filter(STATUS.eq("active"))
    .fetch_all_as::<UserRow>(&pool)
    .await?;
```

Scalar queries use `fetch_one_scalar::<T>()`; raw SQL uses `raw("... ? ...")`
with `?` placeholders. `??` renders a literal question mark.

## Server-Owned SQL Shape

Rust code owns joins, CTEs, subqueries, set queries, aggregates, windows, locks,
and write conflict handling. Client JSON never defines these shapes.

```rust
let paid_orders = cte(
    "paid_orders",
    select(schema::orders::table())
        .column(schema::orders::USER_ID)
        .filter(schema::orders::STATUS.eq("paid")),
    vec![*schema::orders::USER_ID.meta],
);
let paid_orders_source = paid_orders.source().alias("po");
let u = schema::users::alias("u");

let rows = select(&u)
    .with(paid_orders)
    .join(
        paid_orders_source,
        u.id().eq_field(schema::orders::USER_ID.at("po")),
    )
    .column(u.email())
    .filter(u.email().contains("@example.com"))
    .order_desc(u.id())
    .fetch_all_as::<UserRow>(&pool)
    .await?;
```

Typed helpers cover the common Postgres clauses: `distinct_on`, `group_by`,
`having`, row locks, `union_all`, `in_subquery`, `count_distinct`, aggregate
`FILTER`, window functions, array/jsonb/range predicates, `insert(...).from_select(...)`,
and `on_conflict(...).do_update_set(...)`. REST-style pagination stays in
application code; the REST sample shows `limit` / `offset` plus `Select::count()`
for a matching count query.

SQL expression helpers live in `rqb::dsl`, outside the prelude, so broad names
like `left`, `right`, `lower`, `replace`, `row`, and `array` do not pollute every
service module.

```rust
use rqb::dsl::{coalesce, date_trunc};
use rqb::prelude::*;
```

## JSON Search

`SearchRequest` is for client-controlled search parameters only. It cannot
define tables, joins, raw SQL, subqueries, writes, computed projections, or
response fields.

```json
{
  "filter": {
    "and": [
      { "field": "status", "operator": "equals", "value": "paid" },
      { "field": "total_cents", "operator": "gte", "value": 5000 }
    ]
  },
  "sort": [{ "field": "created_at", "dir": "desc" }],
  "limit": 20,
  "offset": 0
}
```

Apply it to a trusted Rust query:

```rust
let query = select(order_search_view())
    .filter(ORGANIZATION_ID.eq(current_org_id))
    .request(search_request)?
    .build()?;
```

Server filters are preserved and combined with the request filter using `AND`.
Only fields with `Meta::json(...)` are visible to JSON requests.

## Writes

Writes use field assignments or derive-generated assignments. There is no
serde write bridge.

```rust
let created = insert(users())
    .set(ID.set(user_id))
    .set(EMAIL.set("ada@example.com"))
    .set(STATUS.set("active"))
    .returning(ID)
    .fetch_one_scalar::<Uuid>(&pool)
    .await?;

update(users())
    .set(STATUS.set("disabled"))
    .filter(ID.eq(user_id))
    .execute(&pool)
    .await?;
```

With generated schema modules, request DTOs can derive write mappings:

```rust
#[derive(rqb::Insertable)]
#[rqb(table = schema::users)]
struct NewUser {
    email: String,
    status: String,
}

let created = insert(schema::users::table())
    .set(schema::users::ID.set(user_id))
    .values(&new_user)
    .returning(schema::users::ID)
    .fetch_one_scalar::<Uuid>(&pool)
    .await?;
```

`#[derive(rqb::Changeset)]` maps `Option<T>` fields as patch fields: `Some`
sets the column, `None` leaves it unchanged.

`DELETE` without a filter is rejected during validation.

## Transactions

rqb executes through sqlx. Use `tx!` when several statements must commit or
roll back together:

```rust
tx!(&pool, |conn| {
    let created_id = insert(users())
        .set(ID.set(user_id))
        .set(EMAIL.set("ada@example.com"))
        .returning(ID)
        .fetch_one_scalar::<Uuid>(conn)
        .await?;
    Ok(created_id)
})
.await?;
```

## CLI

`rqb-cli` introspects Postgres and writes a compact `rqb::schema!` module. The
macro expands to `Meta`, `Field<T>`, `FIELDS`, and `table()` / `view()` items.

```bash
cargo run -p rqb-cli -- generate \
  --database-url "$DATABASE_URL" \
  --schema public \
  --out src/schema.rs
```

Known sqlx-supported Postgres types generate typed `Field<T>` constants.
Unknown extension types stay raw-only metadata: they can be part of server-owned
SQL shape, but they are hidden from JSON requests by default.

Generated field names match database column names. HTTP JSON casing belongs in
application DTOs, not in generated schema metadata.

## Crates

- `rqb`: typed AST, renderer, params, execution helpers, and public API.
- `rqb-macros`: procedural macros re-exported by `rqb`.
- `rqb-cli`: schema introspection and code generation, not published.

## Checks

```bash
cargo fmt --all --check
cargo test --workspace --no-default-features
cargo clippy --workspace --all-targets --no-default-features -- -D warnings
```

## License

Licensed under either Apache-2.0 or MIT, at your option.
