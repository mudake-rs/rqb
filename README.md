# rqb — Rust Query Builder

Rust Query Builder for Postgres, built on sqlx.

rqb is not an ORM. It builds parameterized SQL from server-owned query shape and
small field metadata. Typed Rust values go straight into sqlx bind arguments;
JSON search is a constrained adapter for filters, sort, limit, and offset.

## TL;DR

Start with [`samples`](samples). They are short, compile-checked, and show the
actual API faster than prose.

## Contents

- [Why rqb](#why-rqb)
- [Status](#status)
- [Install](#install)
- [Basic Query](#basic-query)
- [Execution](#execution)
- [Server-Owned SQL Shape](#server-owned-sql-shape)
- [JSON Search](#json-search)
- [Raw Escape Hatches](#raw-escape-hatches)
- [Error Handling](#error-handling)
- [Writes](#writes)
- [Transactions](#transactions)
- [CLI](#cli)
- [Migrations And Schema Drift](#migrations-and-schema-drift)
- [Crates](#crates)
- [Checks](#checks)
- [License](#license)

## Why rqb

- You write Postgres, not generic SQL. rqb does not hide the dialect.
- Query shape is owned by Rust code and validated before SQL is rendered.
- Client JSON can filter, sort, limit, and offset only through exposed metadata;
  it cannot define joins, raw SQL, CTEs, writes, or projections.
- Values bind through sqlx Postgres arguments directly. Writes do not pass
  through a serde JSON bridge.

rqb gives Rust code typed field metadata, typed bind paths, and pre-render
validation. It is not a full compile-time SQL type system: some shape and
operator mistakes are rejected at `.build()?`, and PostgreSQL remains the final
authority for name resolution, constraints, permissions, and query planning.

## Status

Pre-1.0. The repository is public and the supported distribution path is the
GitHub repository. APIs are still allowed to break when that makes the library
simpler or clearer.

The core builder targets Postgres 14+ for ordinary application queries. Some
helpers expose newer server features such as `MERGE` branches from Postgres 17
and UUIDv7 / old-new DML `RETURNING` from Postgres 18. The integration suite
runs against Postgres 18.

## Install

rqb is distributed as a git dependency:

```toml
[dependencies]
rqb = { git = "https://github.com/mudake-rs/rqb" }
chrono = "0.4.45"
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.150"
sqlx = { version = "0.9.0", features = [
    "postgres",
    "derive",
    "uuid",
    "chrono",
    "json",
    "runtime-tokio",
    "tls-rustls",
] }
uuid = "1.23.4"
```

For reproducible application builds, pin a commit:

```toml
rqb = { git = "https://github.com/mudake-rs/rqb", rev = "<commit>" }
```

`uuid`, `chrono`, JSON, numeric, ranges, arrays, and other Postgres values are
accepted when the Rust type implements sqlx `Encode` and `Type` for Postgres.
Direct expression literals cover common bind types such as `Uuid`, chrono
date/time values, `PgInterval` / durations, `BigDecimal`, `Vec<u8>`, and
`serde_json::Value`; `param(value)` remains the explicit fallback for any other
sqlx-supported value.

## Basic Query

Generated schema modules are normal Rust modules. `table()` / `view()` provide
the source metadata, uppercase constants are typed fields, and `alias("u")`
returns an alias-bound handle for join-heavy queries.

Use alias handles as soon as a query or write has more than one source. rqb can
validate field capabilities and query shape, but PostgreSQL owns final name
resolution; unqualified columns that exist in both sources can still fail at
execution with an ambiguous-column error. The same applies to `MERGE`: alias the
target and incoming source when the `ON` clause or branch conditions compare
same-named columns.

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
    .columns((app_users::ID, app_users::EMAIL))
    .filter(app_users::STATUS.eq("active"))
    .order_asc(app_users::EMAIL)
    .limit(20)
    .build()?;
```

Examples below use `schema::users::*` as a placeholder for whatever module your
generated schema lives in. Samples in `samples/` use imports such as
`use rqb_sample_schema::app_users as users;`.

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
    .columns((schema::users::ID, schema::users::EMAIL))
    .filter(schema::users::STATUS.eq("active"))
    .fetch_all_as::<UserRow>(&pool)
    .await?;
```

Service functions can usually accept `&PgPool`. When a query must be reused
inside a transaction, prefer reusing the query shape instead of making the async
service function generic over an executor:

```rust
fn find_user_query(id: Uuid) -> Select {
    select(schema::users::table())
        .filter(schema::users::ID.eq(id))
}

async fn find_user(pool: &PgPool, id: Uuid) -> rqb::Result<UserRow> {
    find_user_query(id).fetch_one_as::<UserRow>(pool).await
}

tx!(&pool, |conn| {
    let user = find_user_query(id)
        .fetch_one_as::<UserRow>(&mut *conn)
        .await?;
    Ok(user)
})
.await?;
```

Use `impl PgExecutor<'_>` for small helpers that should execute directly from
both pool-backed code and transaction code:

```rust
async fn cancel_open_orders(db: impl PgExecutor<'_>, user_id: Uuid) -> rqb::Result<u64> {
    update(schema::orders::table())
        .set(schema::orders::STATUS.set("canceled"))
        .filter(schema::orders::USER_ID.eq(user_id))
        .filter(schema::orders::STATUS.eq("open"))
        .execute(db)
        .await
}
```

When passing a sqlx transaction or pool connection to an executor helper, use
sqlx's reborrow pattern: `&mut *tx` or `&mut *conn`.

Statement convenience methods such as `fetch_all_as::<T>(&pool)` build,
validate, and render the SQL for that call. If a hot path executes the same
server-owned query shape repeatedly, build once and reuse the `BuiltQuery`:

```rust
let query = select(schema::users::table())
    .columns((schema::users::ID, schema::users::EMAIL))
    .filter(schema::users::STATUS.eq("active"))
    .build()?;

let rows = query.fetch_all_as::<UserRow>(&pool).await?;
```

Scalar queries use `fetch_one_scalar::<T>()`; raw SQL uses `raw("... ? ...")`
with `?` placeholders. `??` renders a literal question mark.

For HTTP response streams, pass a cloned pool handle into the owned streaming
helpers: `fetch_stream_pool`, `fetch_stream_pool_as::<T>()`, or
`fetch_stream_pool_scalar::<T>()`. The returned stream owns the built query and
pool handle. `BuiltQuery::fetch_stream*` remains available when you build once
and keep the built query alive yourself. HTTP handlers should still choose their
own chunking policy; rqb yields rows, application code formats response chunks.

## Server-Owned SQL Shape

Rust code owns joins, CTEs, subqueries, set queries, aggregates, windows, locks,
and write conflict handling. Client JSON never defines these shapes.

```rust
use rqb::dsl::{exists, sum};
use rqb::prelude::*;

let paid_orders = select(schema::orders::table())
    .columns((schema::orders::USER_ID, schema::orders::TOTAL_CENTS))
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
    .item(sum(schema::orders::TOTAL_CENTS.at("po")).alias("paid_total"))
    .group_by(u.id())
    .fetch_all_as::<UserRow>(&pool)
    .await?;
```

Typed helpers cover the common Postgres clauses: `distinct_on`, `group_by`,
`having`, row locks, `union_all`, `in_subquery`, `count_distinct`, aggregate
`FILTER`, window functions, array/jsonb/range predicates, conditional
`filter_if(...)` / `filter_option(...)` / `or_filter_option(...)` /
`set_if(...)` / `set_option(...)` helpers, `set_many((...))`, row-value
comparisons for cursor pagination, `insert(...).columns((...)).from_select(...)`,
`values_source(...)`, `generate_series_source(...)`,
`on_conflict((col_a, col_b)).do_update_excluded((...))`, and
`merge_into(...).when_matched_if(...).update(...)`. MERGE actions are validated
against Postgres `WHEN` clause rules before rendering. REST-style pagination stays
in application code; the REST sample shows `limit` / `offset` plus
`Select::count()` for a matching count query, cursor pagination, and
pool-owned streaming CSV responses into axum `Body::from_stream`.

`Select::count()` is a matching-row count helper, not a locked-query replay. It
removes ordering, page limits, `FETCH`, and row-lock clauses before wrapping the
query in `count(*)`. For `FOR UPDATE SKIP LOCKED`-style workflows, count the
locked result set explicitly in server-owned SQL if lock semantics matter.

Postgres `MERGE ... RETURNING` reports rows affected by each executed action,
including rows deleted by `WHEN NOT MATCHED BY SOURCE THEN DELETE`. If an API
needs the final table state after that branch, run a follow-up `SELECT` with the
same server-owned scope.

Grouped analytics can make non-null source columns nullable in result rows.
`ROLLUP`, `CUBE`, and `GROUPING SETS` emit subtotal rows by replacing grouped
dimension values with `NULL`, so downstream `sqlx::FromRow` structs should use
`Option<T>` for those projected dimensions.

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

SQL expression helpers are available as qualified `rqb::...` calls, but they
live outside the prelude so broad names like `left`, `right`, `lower`,
`replace`, `row`, and `array` do not pollute every service module. Use
`rqb::dsl::*` for short query modules, or import the exact helpers a module
needs:

```rust
use rqb::dsl::{coalesce, count_all, date_trunc_part, sum, DatePart};
use rqb::prelude::*;
```

Common helper families in the flat catalog:

| Family | Examples |
| --- | --- |
| Boolean predicates | `and`, `or`, `not`, `exists`, `true_`, `false_` |
| Aggregates | `count_all`, `sum`, `sum_distinct`, `avg_distinct`, `jsonb_agg_object`, `percentile_cont` |
| Arrays | `array`, `array_length`, `array_position`, `trim_array`, `unnest` |
| Date and time | `now`, `date_trunc`, `date_trunc_part`, `date_bin`, `to_char`, `isfinite` |
| Full-text search | `to_tsvector`, `phraseto_tsquery`, `ts_rank`, `ts_headline` |
| JSON/JSONB | `json_build_object`, `jsonb_build_object`, `jsonb_pretty`, `array_to_json` |
| Math | `round`, `sqrt`, `pow`, `random_between`, `width_bucket` |
| Range | `range_lower`, `range_upper`, `range_merge`, `multirange_merge`, `isempty`, `lower_inc` |
| Scalar expressions | `case`, `coalesce`, `null`, `literal`, `greatest`, `current_user`, `scalar_subquery` |
| Set-returning sources | `generate_series_source`, `unnest_source`, `json_each_source`, `regexp_split_to_table_source`, `values_source` |
| Text | `lower`, `format`, `translate`, `repeat`, `octet_length`, `encode` |
| UUID | `uuidv7`, `uuid_extract_timestamp`, `gen_random_uuid` |
| Window functions | `window`, `row_number`, `rank`, `lag`, `preceding`, `unbounded_preceding` |

When PostgreSQL expects a stable SQL vocabulary literal rather than user data,
use typed helpers or `literal(...)` instead of a bind parameter:

```rust
use rqb::dsl::{DatePart, date_trunc_part, literal};

let day = date_trunc_part(DatePart::Day, schema::orders::CREATED_AT);
let month = rqb::date_trunc(literal("month"), schema::orders::CREATED_AT);
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
    .apply_search(search_request)?
    .build()?;
```

Server filters are preserved and combined with the request filter using `AND`.
Only fields with `Meta::json(...)` are visible to JSON requests. Operators are
gated by field capabilities: equality/null tests require equality capability,
sort requires ordering capability, and LIKE/regex/text-pattern operators require
text-pattern capability such as `OpSet::text()`. Client-supplied LIKE and regex
patterns are capped at 1024 Unicode scalar values; public APIs should still set
a database `statement_timeout` appropriate for their workload.

> Tenant and permission scope: install tenant, user, RBAC, and soft-delete
> filters before calling `apply_search`. If a request is allowed to own the
> entire search clause, start from a fresh `select(...)` and apply it there.

Search failures are ordinary `rqb::Error` variants, so API boundaries can return
stable client errors without parsing strings:

| Error | Usual HTTP shape |
| --- | --- |
| `InvalidSearchField` | `400`, unknown field |
| `SearchFieldNotExposed` | `400`, field exists but is not exposed to JSON search |
| `InvalidSearchOperator` | `400`, operator is not allowed for that field |
| `InvalidSearchValue` | `400`, JSON value has the wrong shape for the field kind |
| `InvalidSort` | `400`, field is visible but not sortable |
| `EmptySearchLogical` | `400`, `and` / `or` group is empty |

See `samples/json-search` for exact payloads and error mapping.

## Raw Escape Hatches

The typed DSL is not meant to mirror every PostgreSQL catalog function. For
simple unlisted functions, use `function(...)`, `aggregate(...)`, or
`function_source(...)`. For PostgreSQL syntax that does not fit those shapes,
rqb exposes raw constructors at the exact AST slot:

| Helper | Returns | Use for |
| --- | --- | --- |
| `raw(...)` | `RawStmt` | A full server-owned statement |
| `raw_expr(...)` | `ValueExpr` | A projection, assignment value, or comparison side |
| `raw_predicate(...)` | `BoolExpr` | A `WHERE`, `ON`, or `HAVING` predicate |
| `raw_source(...)` | `Source` | A derived table or set-returning expression in `FROM` |

Raw fragments use rqb `?` placeholders outside SQL quoted contexts. Question
marks inside single-quoted strings, dollar-quoted bodies, quoted identifiers,
and comments stay literal. Escape literal question marks in SQL operator
positions as `??`, for example Postgres JSONB `?` operators. Raw fragments are
validated for bind-count mismatches and numbered together with the surrounding
typed query:

```rust
use rqb::prelude::*;

let extension_score = raw_expr(
    "custom_extension_score(payload, ?::text)",
    [Param::typed("strict".to_owned())],
)
.alias("extension_score");

let extension_rows = raw_source(
    "SELECT * FROM custom_extension_scan(?::text)",
    "ext",
    vec![
        Param::typed("tenant-42".to_owned()),
    ],
    rqb::field!("id": uuid => uuid::Uuid, equality),
);
```

Any raw fragment marks the whole built query as non-cacheable, so execution uses
`sqlx` with persistent prepared-statement caching disabled for that query. Keep
raw fragments for server-owned SQL that genuinely needs them; prefer typed DSL
helpers when they express the same stable SQL shape.

Generated schemas keep unknown extension columns as raw-only metadata constants
such as `EMBEDDING_META`. Use `*_META.expr()` / `*_META.at("alias")` with
`op(...)`, `cast(...)`, or raw helpers when a column is deliberately outside
rqb's typed `Field<T>` catalog.

## Error Handling

rqb returns one structured `Error` enum for validation failures, sqlx execution
failures, and mapped Postgres SQLSTATE errors. Application code should match on
variants, not parse database message strings.

The usual HTTP mapping is:

- `NotFound` -> `404 Not Found`.
- `UniqueViolation` / `ExclusionViolation` -> `409 Conflict`.
  Use `constraint_name()` when logging or building domain-specific messages.
- `ForeignKeyViolation`, `RestrictViolation`, `NotNullViolation`, and
  `CheckViolation` -> `400 Bad Request`.
- `InvalidSearchField`, `SearchFieldNotExposed`, `InvalidSearchOperator`,
  `InvalidSearchValue`, `EmptySearchLogical`, and `InvalidSort` ->
  `400 Bad Request`.
- `SerializationFailure`, `DeadlockDetected`, and connection failures ->
  `503 Service Unavailable` or a retry response. `error.is_retryable()` returns
  true for these retryable cases.
- `QueryCanceled` -> `504 Gateway Timeout` or request timeout, depending on
  who canceled the query.
- `InsufficientPrivilege` -> `403 Forbidden`.
- Builder-shape errors such as `DeleteWithoutFilter`, `RawBindMismatch`,
  `InvalidInsertShape`, `InvalidCteShape`, and `InvalidRowShape` are usually
  `500 Internal Server Error` unless they came directly from a JSON request.

See `samples/error-handling` for standalone matching examples and
`samples/rest-api/src/error.rs` for the same pattern inside an axum
`IntoResponse` boundary.

## Writes

Writes use field assignments or derive-generated assignments. There is no
serde write bridge.

```rust
let created = insert(schema::users::table())
    .set_many((
        schema::users::ID.set(user_id),
        schema::users::EMAIL.set("ada@example.com"),
        schema::users::STATUS.set("active"),
    ))
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

Use `set_null()` when an application intentionally writes SQL `NULL`:

```rust
update(schema::invoices::table())
    .set(schema::invoices::PAID_AT.set_null())
    .filter(schema::invoices::ID.eq(invoice_id))
    .execute(&pool)
    .await?;
```

rqb renders the assignment; PostgreSQL still enforces `NOT NULL` constraints at
execution time.

Conditional write helpers keep service code linear when a field depends on
application state:

```rust
update(schema::users::table())
    .set_option(new_email, |email| schema::users::EMAIL.set(email))
    .set_option(new_status, |status| schema::users::STATUS.set(status))
    .filter(schema::users::ID.eq(user_id))
    .filter_option(current_status, |status| schema::users::STATUS.eq(status))
    .execute(&pool)
    .await?;
```

`UPDATE` and `DELETE` also accept CTEs, so write statements can stay in the
typed builder instead of falling back to raw SQL:

```rust
let active_ids = select(schema::users::table())
    .column(schema::users::ID)
    .filter(schema::users::ACTIVE.eq(true))
    .try_into_cte("active_ids")?;
let active_ids_source = active_ids.source().alias("ids");
let u = schema::users::alias("u");

update(&u)
    .with(active_ids)
    .set(schema::users::STATUS.set("active"))
    .from(active_ids_source)
    .filter(u.id().eq_field(schema::users::ID.at("ids")))
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

The derive maps Rust fields to generated schema fields. It does not serialize
the whole DTO through `serde_json`, so database types stay on the sqlx encode
path.

Derives map `snake_case` Rust fields to generated `SHOUTY_SNAKE_CASE` schema
constants by default. Use `#[rqb(field = schema::table::FIELD)]` when a DTO field
name differs from the database column, and `#[rqb(skip)]` for local-only fields.
For `Insertable`, `#[rqb(skip_none)]` skips `None` on an `Option<T>` field;
otherwise the `Option<T>` itself is inserted as the value. `Changeset` always
treats `Option<T>` as patch semantics: `Some` sets the column, `None` leaves it
unchanged.

Upserts can update several columns from `EXCLUDED` without repeating
`set_excluded()` per field:

```rust
insert(schema::products::table())
    .values(&product)
    .on_conflict(schema::products::SKU)
    .do_update_excluded((
        schema::products::NAME,
        schema::products::PRICE_CENTS,
        schema::products::ATTRIBUTES,
        schema::products::TAGS,
    ))
    .returning_all()
    .fetch_one_as::<ProductRow>(&pool)
    .await?;
```

For sync-style batch inputs, expose the incoming rows as a `values_source`.
`from_select_all(...)` uses that source metadata for both the target column list
and the `SELECT` projection, and `set_from("alias")` copies fields from the
same source in conflict updates:

```rust
let incoming = values_source(
    [(sku, name, price_cents)],
    "incoming",
    (
        schema::products::SKU,
        schema::products::NAME,
        schema::products::PRICE_CENTS,
    ),
);

insert(schema::products::table())
    .from_select_all(incoming)
    .on_conflict(schema::products::SKU)
    .do_update_set((
        schema::products::NAME.set_from("incoming"),
        schema::products::PRICE_CENTS.set_from("incoming"),
    ))
    .execute(&pool)
    .await?;
```

Optimistic locking stays normal update SQL: include both the row id and expected
version in `WHERE`, increment the version in `SET`, and use `RETURNING` to tell
success from a version miss.

```rust
let updated = update(schema::orders::table())
    .set_many((
        schema::orders::STATUS.set("paid"),
        schema::orders::VERSION.set_expr(schema::orders::VERSION.expr().op("+", 1)),
    ))
    .filter(schema::orders::ID.eq(order_id))
    .filter(schema::orders::VERSION.eq(expected_version))
    .returning_all()
    .fetch_optional_as::<OrderRow>(&pool)
    .await?;
```

`DELETE` without a filter is rejected during validation.

## Transactions

rqb executes through sqlx. Use `tx!` when several statements must commit or
roll back together:

```rust
tx!(&pool, |conn| {
    let created_id = insert(schema::users::table())
        .set_many((
            schema::users::ID.set(user_id),
            schema::users::EMAIL.set("ada@example.com"),
        ))
        .returning(schema::users::ID)
        .fetch_one_scalar::<Uuid>(conn)
        .await?;
    Ok(created_id)
})
.await?;
```

Use explicit `pool.begin().await?` / `commit().await?` when you need sqlx
features that do not fit closure scope, such as savepoints, custom isolation
setup, or transaction ownership that crosses helper boundaries.

## CLI

`rqb-cli` introspects Postgres and writes a compact `rqb::schema!` module. The
macro expands to `Meta`, `Field<T>`, `FIELDS`, and `table()` / `view()` items.

Install the CLI from GitHub; the installed binary is `rqb`.

```bash
cargo install --git https://github.com/mudake-rs/rqb rqb-cli
```

The package name is `rqb-cli`; the binary name is `rqb`.

Inside this workspace, run the same binary through `cargo run -p rqb-cli --`.

```bash
rqb generate \
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

The CLI also annotates nullable, generated, identity, and materialized-view
metadata in comments. It does not generate row structs; use `Option<T>` in
`sqlx::FromRow` structs for nullable columns.

## Migrations And Schema Drift

rqb does not run or generate database migrations. Keep schema changes in SQL
migration files and apply them with `sqlx::migrate!`, `sqlx migrate`, refinery,
sqitch, psql, or your deployment system.

The intended flow is:

1. Apply migrations to a real Postgres database.
2. Regenerate the schema module:

   ```bash
   rqb generate \
     --database-url "$DATABASE_URL" \
     --schema public \
     --out src/schema.rs
   ```

3. Commit the generated schema module with the migration.
4. In CI, run the same command with `--check` to catch drift between the
   database schema and the checked-in generated module.

`rqb-cli` introspects the schema that already exists. It does not diff schema
versions, generate `ALTER TABLE`, or decide migration ordering. When a column is
renamed, removed, or changes type, regenerate the schema module and let Rust
compile errors point at stale query code.

## Crates

- `rqb`: typed AST, renderer, params, execution helpers, and public API.
- `rqb-macros`: procedural macros re-exported by `rqb`.
- `rqb-cli`: schema introspection and code generation.

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
