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

Generated schema modules are normal Rust modules. `table()` / `view()` provide
the source metadata, uppercase constants are typed fields, and `alias("u")`
returns an alias-bound handle for join-heavy queries.

```rust
use rqb::prelude::*;
use uuid::Uuid;

rqb::schema! {
    table public.app_users {
        id: uuid = Uuid,
        email: text = String,
        status: text = String,
    }
}

let query = select(app_users::table())
    .column(app_users::ID)
    .column(app_users::EMAIL)
    .filter(app_users::STATUS.eq("active"))
    .order_asc(app_users::EMAIL)
    .limit(20)
    .build()?;
```

`select(table())` does not render `SELECT *`. It renders the known root fields
from metadata, so SQL output stays explicit and stable.

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

let rows = select(schema::users::table())
    .column(schema::users::ID)
    .column(schema::users::EMAIL)
    .filter(schema::users::STATUS.eq("active"))
    .fetch_all_as::<UserRow>(&pool)
    .await?;
```

Service functions usually accept `impl PgExecutor<'e>` so the same query can run
against `&PgPool`, `&mut PgConnection`, or a transaction connection.

Scalar queries use `fetch_one_scalar::<T>()`; raw SQL uses `raw("... ? ...")`
with `?` placeholders. `??` renders a literal question mark.

## Server-Owned SQL Shape

Rust code owns joins, CTEs, subqueries, set queries, aggregates, windows, locks,
and write conflict handling. Client JSON never defines these shapes.

```rust
use rqb::dsl::{exists, sum};
use rqb::prelude::*;

let paid_orders = select(schema::orders::table())
    .column(schema::orders::USER_ID)
    .column(schema::orders::TOTAL_CENTS)
    .filter(schema::orders::STATUS.eq("paid"))
    .try_into_cte("paid_orders")?;

let po = paid_orders.source().alias("po");
let u = schema::users::alias("u");

let rows = select(&u)
    .with(paid_orders)
    .join(po, u.id().eq_field(schema::orders::USER_ID.at("po")))
    .filter(exists(
        select(schema::orders::table())
            .column(schema::orders::ID)
            .filter(schema::orders::USER_ID.eq_field(u.id())),
    ))
    .agg(sum(schema::orders::TOTAL_CENTS.at("po")).alias("paid_total"))
    .group_by(u.id())
    .fetch_all_as::<UserRow>(&pool)
    .await?;
```

Typed helpers cover the common Postgres clauses: `distinct_on`, `group_by`,
`having`, row locks, `union_all`, `in_subquery`, `count_distinct`, aggregate
`FILTER`, window functions, array/jsonb/range predicates, `insert(...).from_select(...)`,
`on_conflict((col_a, col_b)).do_update_set(...)`, and
`merge_into(...).when_matched_if(...).update(...)`. REST-style pagination stays
in application code; the REST sample shows `limit` / `offset` plus
`Select::count()` for a matching count query.

For derived sources, rqb needs exposed field metadata. `Select::try_into_cte`
and `Select::try_into_source` infer it from explicit field projections.
Computed columns can use `rqb::field!`:

```rust
use rqb::dsl::{case, count_all};

let item_count = rqb::field!("item_count": int8 => i64, ordered);

let order_size = case()
    .when(schema::orders::TOTAL_CENTS.gte(10_000), "large")
    .else_("standard");
```

Raw SQL, computed columns with custom aliases, and renamed projections can still
use the explicit `cte(...)`, `subquery(...)`, or `raw_source(...)` constructors.

SQL expression helpers live in `rqb::dsl`, outside the prelude, so broad names
like `left`, `right`, `lower`, `replace`, `row`, and `array` do not pollute every
service module. Import from the grouped modules when autocomplete noise matters:

```rust
use rqb::dsl::agg::{count_all, sum};
use rqb::dsl::date::date_trunc;
use rqb::dsl::scalar::coalesce;
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
let query = select(schema::order_search_view::view())
    .filter(schema::order_search_view::ORGANIZATION_ID.eq(current_org_id))
    .request(search_request)?
    .build()?;
```

Server filters are preserved and combined with the request filter using `AND`.
Only fields with `Meta::json(...)` are visible to JSON requests.

## Writes

Writes use field assignments or derive-generated assignments. There is no
serde write bridge.

```rust
let created = insert(schema::users::table())
    .set(schema::users::ID.set(user_id))
    .set(schema::users::EMAIL.set("ada@example.com"))
    .set(schema::users::STATUS.set("active"))
    .returning(schema::users::ID)
    .fetch_one_scalar::<Uuid>(&pool)
    .await?;

update(schema::users::table())
    .set(schema::users::STATUS.set("disabled"))
    .filter(schema::users::ID.eq(user_id))
    .execute(&pool)
    .await?;
```

Computed writes use `set_expr(...)`, and PostgreSQL 18 `RETURNING old.field` /
`new.field` is available on generated fields:

```rust
let changed = update(schema::users::table())
    .set(schema::users::LOGIN_COUNT.set_expr(
        schema::users::LOGIN_COUNT.expr().op("+", 1),
    ))
    .filter(schema::users::ID.eq(user_id))
    .returning_item(schema::users::LOGIN_COUNT.old_value().alias("old_login_count"))
    .returning_item(schema::users::LOGIN_COUNT.new_value().alias("new_login_count"))
    .fetch_one_as::<LoginCountChange>(&pool)
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

The derive maps Rust fields to generated schema fields. It does not serialize
the whole DTO through `serde_json`, so database types stay on the sqlx encode
path.

`#[derive(rqb::Changeset)]` maps `Option<T>` fields as patch fields: `Some`
sets the column, `None` leaves it unchanged.

`DELETE` without a filter is rejected during validation.

## Transactions

rqb executes through sqlx. Use `tx!` when several statements must commit or
roll back together:

```rust
tx!(&pool, |conn| {
    let created_id = insert(schema::users::table())
        .set(schema::users::ID.set(user_id))
        .set(schema::users::EMAIL.set("ada@example.com"))
        .returning(schema::users::ID)
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
SQL shape, but they are hidden from JSON requests by default. Server-owned
extension operators can be built from raw-only metadata:

```rust
let embedding = vector_documents::EMBEDDING_META.expr();
let distance = embedding.op("<->", rqb::dsl::param("[0.1,0.2,0.3]".to_owned()).cast("vector"));
```

Generated field names match database column names. HTTP JSON casing belongs in
application DTOs, not in generated schema metadata.

## Crates

- `rqb`: typed AST, renderer, params, execution helpers, and public API.
- `rqb-macros`: procedural macros re-exported by `rqb`.
- `rqb-cli`: schema introspection and code generation, not published.

## Checks

```bash
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
make verify
```

## License

Licensed under either Apache-2.0 or MIT, at your option.
